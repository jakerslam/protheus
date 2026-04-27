#!/usr/bin/env node
/* eslint-disable no-console */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

interface TierPolicy {
  priority: number;
  retention_days: number | null;
  cleanup_action: string;
  description: string;
  path_regex: string[];
}

interface LifecyclePolicy {
  schema_version: number;
  policy_id: string;
  scan_roots: string[];
  required_tiers: string[];
  protected_statuses: string[];
  resolved_statuses: string[];
  tiers: Record<string, TierPolicy>;
}

interface ArtifactRow {
  path: string;
  tier: string;
  priority: number;
  cleanup_action: string;
  age_days: number;
  size_bytes: number;
  active_findings: number;
  resolved_findings: number;
  malformed_rows: number;
  planned_action: string;
  reason: string;
}

const DEFAULT_POLICY = 'tests/tooling/config/eval_feedback_lifecycle_policy.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/eval_feedback_lifecycle_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/EVAL_FEEDBACK_LIFECYCLE_GUARD_CURRENT.md';
const MAX_INLINE_INSPECTION_BYTES = 64 * 1024 * 1024;
const STREAM_CHUNK_BYTES = 1024 * 1024;
const STATUS_SCAN_CARRY_BYTES = 512;

interface Args {
  policyPath: string;
  outJson: string;
  outMarkdown: string;
  strict: boolean;
}

function readArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  return {
    policyPath: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY, 400),
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
    strict: common.strict,
  };
}

function readJson<T>(filePath: string): T {
  return JSON.parse(fs.readFileSync(filePath, 'utf8')) as T;
}

function normalizePath(filePath: string): string {
  return filePath.split(path.sep).join('/');
}

function walk(root: string, out: string[]): void {
  if (!fs.existsSync(root)) return;
  const entries = fs.readdirSync(root, { withFileTypes: true });
  for (const entry of entries) {
    if (entry.name === 'node_modules' || entry.name === '.git' || entry.name === 'target') continue;
    const next = path.join(root, entry.name);
    if (entry.isDirectory()) {
      walk(next, out);
    } else if (entry.isFile()) {
      out.push(next);
    }
  }
}

function classify(policy: LifecyclePolicy, filePath: string): string | null {
  const normalized = normalizePath(filePath);
  const matches: Array<{ tier: string; priority: number }> = [];
  for (const [tier, tierPolicy] of Object.entries(policy.tiers)) {
    for (const pattern of tierPolicy.path_regex) {
      if (new RegExp(pattern).test(normalized)) {
        matches.push({ tier, priority: tierPolicy.priority });
        break;
      }
    }
  }
  matches.sort((a, b) => b.priority - a.priority);
  return matches[0]?.tier ?? null;
}

function statusesFromValue(value: unknown, statuses: string[]): number {
  let count = 0;
  const walkValue = (input: unknown): void => {
    if (Array.isArray(input)) {
      for (const item of input) walkValue(item);
      return;
    }
    if (!input || typeof input !== 'object') return;
    const obj = input as Record<string, unknown>;
    const status = typeof obj.status === 'string' ? obj.status.toLowerCase() : '';
    if (statuses.includes(status)) count += 1;
    for (const key of ['findings', 'issues', 'issue_candidates', 'items', 'feedback', 'drafts']) {
      if (key in obj) walkValue(obj[key]);
    }
  };
  walkValue(value);
  return count;
}

function inspectLargeJsonLikeByStatusScan(filePath: string, policy: LifecyclePolicy): Pick<ArtifactRow, 'active_findings' | 'resolved_findings' | 'malformed_rows'> {
  let active = 0;
  let resolved = 0;
  let carry = '';
  const buffer = Buffer.alloc(STREAM_CHUNK_BYTES);
  const fd = fs.openSync(filePath, 'r');
  const scanStatuses = (text: string): void => {
    const re = /"status"\s*:\s*"([^"]+)"/gi;
    let match: RegExpExecArray | null;
    while ((match = re.exec(text)) !== null) {
      const status = match[1].toLowerCase();
      if (policy.protected_statuses.includes(status)) active += 1;
      if (policy.resolved_statuses.includes(status)) resolved += 1;
    }
  };
  try {
    for (;;) {
      const bytesRead = fs.readSync(fd, buffer, 0, buffer.length, null);
      if (bytesRead <= 0) break;
      const text = carry + buffer.toString('utf8', 0, bytesRead);
      const stableEnd = Math.max(0, text.length - STATUS_SCAN_CARRY_BYTES);
      scanStatuses(text.slice(0, stableEnd));
      carry = text.slice(stableEnd);
    }
    scanStatuses(carry);
  } finally {
    fs.closeSync(fd);
  }
  return { active_findings: active, resolved_findings: resolved, malformed_rows: 0 };
}

function inspectJsonLike(filePath: string, policy: LifecyclePolicy): Pick<ArtifactRow, 'active_findings' | 'resolved_findings' | 'malformed_rows'> {
  const empty = { active_findings: 0, resolved_findings: 0, malformed_rows: 0 };
  const stat = fs.statSync(filePath);
  if (stat.size > MAX_INLINE_INSPECTION_BYTES) {
    return inspectLargeJsonLikeByStatusScan(filePath, policy);
  }
  const text = fs.readFileSync(filePath, 'utf8');
  if (!text.trim()) return empty;
  try {
    if (filePath.endsWith('.jsonl')) {
      let active = 0;
      let resolved = 0;
      let malformed = 0;
      for (const line of text.split(/\r?\n/)) {
        if (!line.trim()) continue;
        try {
          const parsed = JSON.parse(line) as Record<string, unknown>;
          const status = typeof parsed.status === 'string' ? parsed.status.toLowerCase() : '';
          if (policy.protected_statuses.includes(status)) active += 1;
          if (policy.resolved_statuses.includes(status)) resolved += 1;
        } catch {
          malformed += 1;
        }
      }
      return { active_findings: active, resolved_findings: resolved, malformed_rows: malformed };
    }
    const parsed = JSON.parse(text);
    return {
      active_findings: statusesFromValue(parsed, policy.protected_statuses),
      resolved_findings: statusesFromValue(parsed, policy.resolved_statuses),
      malformed_rows: Number((parsed as Record<string, unknown>).malformed_finding_count ?? 0),
    };
  } catch {
    return { ...empty, malformed_rows: 1 };
  }
}

function plannedAction(row: Omit<ArtifactRow, 'planned_action' | 'reason'>, tierPolicy: TierPolicy): Pick<ArtifactRow, 'planned_action' | 'reason'> {
  if (row.tier === 'kernel_sentinel_finding' && row.active_findings > 0) {
    return { planned_action: 'preserve_active_kernel_evidence', reason: 'active Kernel Sentinel evidence cannot be cleaned by churn cleanup' };
  }
  if (row.tier === 'accepted_eval_finding' && row.active_findings > 0) {
    return { planned_action: 'preserve_active_eval_finding', reason: 'accepted eval feedback must survive until resolved or superseded' };
  }
  if (tierPolicy.retention_days === null) {
    return { planned_action: tierPolicy.cleanup_action, reason: 'durable evidence tier has no age-based deletion' };
  }
  if (row.age_days > tierPolicy.retention_days) {
    return { planned_action: tierPolicy.cleanup_action, reason: `artifact is older than ${tierPolicy.retention_days} day retention window` };
  }
  return { planned_action: 'keep_until_retention_expires', reason: `artifact remains inside ${tierPolicy.retention_days} day retention window` };
}

function writeMarkdown(outPath: string, payload: Record<string, unknown>, rows: ArtifactRow[], failures: string[]): void {
  const lines = [
    '# Eval Feedback Lifecycle Guard',
    '',
    `- ok: \`${failures.length === 0}\``,
    `- scanned_artifact_count: \`${rows.length}\``,
    `- failure_count: \`${failures.length}\``,
    '',
    '## Tier summary',
  ];
  const summary = payload.summary as Record<string, { count: number; active_findings: number; planned_cleanup: number }>;
  for (const [tier, row] of Object.entries(summary)) {
    lines.push(`- ${tier}: count=${row.count}; active=${row.active_findings}; planned_cleanup=${row.planned_cleanup}`);
  }
  if (failures.length > 0) {
    lines.push('', '## Failures');
    for (const failure of failures) lines.push(`- ${failure}`);
  }
  lines.push('', '## Planned cleanup / preservation actions');
  for (const row of rows.slice(0, 80)) {
    lines.push(`- ${row.planned_action}: ${row.path} (${row.reason})`);
  }
  writeTextArtifact(outPath, `${lines.join('\n')}\n`);
}

function main(): void {
  const { policyPath, outJson, outMarkdown, strict } = readArgs(process.argv.slice(2));
  const policy = readJson<LifecyclePolicy>(policyPath);
  const failures: string[] = [];

  for (const tier of policy.required_tiers) {
    if (!policy.tiers[tier]) failures.push(`missing_required_tier:${tier}`);
  }
  if ((policy.tiers.raw_eval_trace?.priority ?? 0) >= (policy.tiers.issue_candidate?.priority ?? 0)) {
    failures.push('raw_eval_trace_priority_must_be_lower_than_issue_candidate');
  }
  if ((policy.tiers.issue_candidate?.priority ?? 0) >= (policy.tiers.accepted_eval_finding?.priority ?? 0)) {
    failures.push('issue_candidate_priority_must_be_lower_than_accepted_eval_finding');
  }
  if ((policy.tiers.accepted_eval_finding?.priority ?? 0) >= (policy.tiers.kernel_sentinel_finding?.priority ?? 0)) {
    failures.push('accepted_eval_finding_priority_must_be_lower_than_kernel_sentinel_finding');
  }

  const files: string[] = [];
  for (const root of policy.scan_roots) walk(root, files);

  const rows: ArtifactRow[] = [];
  for (const file of files) {
    const tier = classify(policy, file);
    if (!tier) continue;
    const stat = fs.statSync(file);
    const tierPolicy = policy.tiers[tier];
    const inspected = inspectJsonLike(file, policy);
    const ageDays = Math.max(0, (Date.now() - stat.mtimeMs) / 86_400_000);
    const base = {
      path: normalizePath(file),
      tier,
      priority: tierPolicy.priority,
      cleanup_action: tierPolicy.cleanup_action,
      age_days: Number(ageDays.toFixed(3)),
      size_bytes: stat.size,
      ...inspected,
    };
    rows.push({ ...base, ...plannedAction(base, tierPolicy) });
  }

  for (const row of rows) {
    if (row.active_findings > 0 && row.tier === 'raw_eval_trace') {
      failures.push(`active_finding_in_non_durable_tier:${row.path}`);
    }
    if (row.tier === 'kernel_sentinel_finding' && row.active_findings > 0 && !row.planned_action.startsWith('preserve_')) {
      failures.push(`active_kernel_finding_not_preserved:${row.path}`);
    }
  }

  const summary: Record<string, { count: number; active_findings: number; resolved_findings: number; planned_cleanup: number; bytes: number }> = {};
  for (const row of rows) {
    summary[row.tier] ??= { count: 0, active_findings: 0, resolved_findings: 0, planned_cleanup: 0, bytes: 0 };
    summary[row.tier].count += 1;
    summary[row.tier].active_findings += row.active_findings;
    summary[row.tier].resolved_findings += row.resolved_findings;
    summary[row.tier].bytes += row.size_bytes;
    if (!row.planned_action.startsWith('preserve_') && row.planned_action !== 'keep_until_retention_expires') summary[row.tier].planned_cleanup += 1;
  }

  const payload = {
    type: 'eval_feedback_lifecycle_guard',
    policy_id: policy.policy_id,
    ok: failures.length === 0,
    strict,
    summary,
    failures,
    artifacts: rows.sort((a, b) => b.priority - a.priority || a.path.localeCompare(b.path)),
  };

  writeMarkdown(outMarkdown, payload, payload.artifacts as ArtifactRow[], failures);

  process.exitCode = emitStructuredResult(payload, {
    outPath: outJson,
    strict,
    ok: payload.ok,
  });
}

main();
