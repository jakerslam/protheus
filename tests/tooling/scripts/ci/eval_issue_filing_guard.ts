#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

const DEFAULT_MONITOR_PATH = 'local/state/ops/eval_agent_chat_monitor/latest.json';
const DEFAULT_POLICY_PATH = 'tests/tooling/config/eval_issue_filing_policy.json';
const DEFAULT_APPROVALS_PATH = 'local/state/ops/eval_agent_chat_monitor/issue_filing_approvals.json';
const DEFAULT_OUT_PATH = 'core/local/artifacts/eval_issue_filing_guard_current.json';
const DEFAULT_OUT_LATEST_PATH = 'artifacts/eval_issue_filing_guard_latest.json';
const DEFAULT_DRAFTS_STATE_PATH = 'local/state/ops/eval_agent_chat_monitor/issue_drafts_latest.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/EVAL_ISSUE_FILING_GUARD_CURRENT.md';

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT_PATH });
  return {
    strict: common.strict,
    monitorPath: cleanText(readFlag(argv, 'monitor') || DEFAULT_MONITOR_PATH, 500),
    policyPath: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY_PATH, 500),
    approvalsPath: cleanText(readFlag(argv, 'approvals') || DEFAULT_APPROVALS_PATH, 500),
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT_PATH, 500),
    outLatestPath: cleanText(readFlag(argv, 'out-latest') || DEFAULT_OUT_LATEST_PATH, 500),
    draftsStatePath: cleanText(readFlag(argv, 'drafts-state') || DEFAULT_DRAFTS_STATE_PATH, 500),
    markdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MARKDOWN_PATH, 500),
  };
}

function readJson(filePath: string): any | null {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function severityRank(raw: string): number {
  const normalized = cleanText(raw, 30).toLowerCase();
  const ranks: Record<string, number> = {
    info: 0,
    low: 1,
    medium: 2,
    high: 3,
    critical: 4,
  };
  return Number(ranks[normalized] ?? 0);
}

function agentIdFromTurnId(raw: unknown): string {
  const text = cleanText(raw, 240);
  const match = text.match(/\bagent-[A-Za-z0-9_-]+\b/);
  return match ? cleanText(match[0], 180) : '';
}

function latestEvidenceRows(rows: any[], limit: number): any[] {
  return [...rows].sort((a, b) => {
    const left = Date.parse(cleanText(a?.ts || a?.turn_id || '', 120)) || 0;
    const right = Date.parse(cleanText(b?.ts || b?.turn_id || '', 120)) || 0;
    return right - left;
  }).slice(0, limit);
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Eval Issue Filing Guard (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report.generated_at || '', 120)}`);
  lines.push(`- ok: ${report.ok === true ? 'true' : 'false'}`);
  lines.push(`- draft_count: ${Number(report.summary?.draft_count || 0)}`);
  lines.push(`- ready_to_file_count: ${Number(report.summary?.ready_to_file_count || 0)}`);
  lines.push('');
  lines.push('## Drafts');
  const drafts = Array.isArray(report.issue_drafts) ? report.issue_drafts : [];
  if (drafts.length === 0) {
    lines.push('- none');
  } else {
    for (const draft of drafts) {
      lines.push(`- ${cleanText(draft?.id || 'issue', 120)} ready=${Boolean(draft?.ready_to_file)} confidence=${Number(draft?.confidence || 0).toFixed(2)} severity=${cleanText(draft?.severity || '', 20)}`);
      if (!Boolean(draft?.ready_to_file)) {
        const blockers = Array.isArray(draft?.blockers) ? draft.blockers : [];
        lines.push(`  blockers=${blockers.map((entry) => cleanText(entry, 80)).join(',') || 'none'}`);
      }
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const root = process.cwd();
  const monitorAbs = path.resolve(root, args.monitorPath);
  const policyAbs = path.resolve(root, args.policyPath);
  const approvalsAbs = path.resolve(root, args.approvalsPath);
  const outLatestAbs = path.resolve(root, args.outLatestPath);
  const draftsStateAbs = path.resolve(root, args.draftsStatePath);
  const markdownAbs = path.resolve(root, args.markdownPath);
  const nowIso = new Date().toISOString();

  const monitor = readJson(monitorAbs) || {};
  const policy = readJson(policyAbs) || {};
  const approvals = readJson(approvalsAbs) || {};
  const issues = Array.isArray(monitor?.issues) ? monitor.issues : [];
  const issueById = new Map<string, any>();
  for (const row of issues) {
    const issueId = cleanText(row?.id || '', 120);
    if (!issueId) continue;
    issueById.set(issueId, row);
  }

  const approvalById = new Map<string, any>();
  const approvalRows = Array.isArray(approvals?.approvals)
    ? approvals.approvals
    : Array.isArray(approvals?.approved_issue_ids)
      ? approvals.approved_issue_ids.map((id: unknown) => ({ id }))
      : [];
  for (const row of approvalRows) {
    const issueId = cleanText(row?.id || row, 120);
    if (!issueId) continue;
    approvalById.set(issueId, row);
  }

  const allowSeverities = new Set(
    (Array.isArray(policy?.allow_severities) ? policy.allow_severities : ['high', 'critical'])
      .map((row: unknown) => cleanText(row, 30).toLowerCase())
      .filter(Boolean),
  );
  const minSeverity = cleanText(policy?.min_severity || 'high', 30).toLowerCase();
  const minSeverityRank = severityRank(minSeverity);
  const minConfidence = Number.isFinite(Number(policy?.min_confidence))
    ? Number(policy.min_confidence)
    : 0.8;
  const requirePersistence = policy?.require_persistence_threshold_met !== false;
  const requireHumanApproval = policy?.require_human_approval !== false;

  const drafts = issues.map((issue: any) => {
    const issueId = cleanText(issue?.id || '', 120);
    const severity = cleanText(issue?.severity || 'medium', 20).toLowerCase();
    const confidence = Number(issue?.confidence || 0);
    const persistenceMet = Boolean(issue?.persistence_threshold_met);
    const approval = approvalById.get(issueId);
    const approvedBy = cleanText(approval?.approved_by || approval?.reviewer || '', 120);
    const blockers: string[] = [];
    if (!allowSeverities.has(severity)) blockers.push('severity_not_allowlisted');
    if (severityRank(severity) < minSeverityRank) blockers.push('severity_below_min');
    if (confidence < minConfidence) blockers.push('confidence_below_min');
    if (requirePersistence && !persistenceMet) blockers.push('persistence_not_met');
    if (requireHumanApproval && !approval) blockers.push('human_approval_missing');

    const evidenceRows = latestEvidenceRows(
      Array.isArray(issue?.evidence) ? issue.evidence : [],
      2,
    );
    const acceptanceCriteria = Array.isArray(issue?.acceptance_criteria)
      ? issue.acceptance_criteria.map((entry: unknown) => cleanText(entry, 260)).filter(Boolean)
      : [];
    const readyToFile = blockers.length === 0;
    const evidence = evidenceRows.slice(0, 2).map((row: any) => ({
      turn_id: cleanText(row?.turn_id || '', 200),
      ts: cleanText(row?.ts || '', 120),
      agent_id: cleanText(row?.agent_id || agentIdFromTurnId(row?.turn_id), 180),
      snippet: cleanText(row?.snippet || '', 260),
    }));
    const relatedAgentIds = Array.from(
      new Set(evidence.map((row) => row.agent_id).filter(Boolean)),
    );
    const title = `[${severity.toUpperCase()}][${issueId}] ${cleanText(issue?.summary || 'Eval issue', 120)}`;
    const bodyLines = [
      `Issue ID: ${issueId}`,
      `Fingerprint: ${cleanText(issue?.issue_fingerprint || 'unknown', 80)}`,
      `Severity: ${severity}`,
      `Confidence: ${confidence.toFixed(2)}`,
      `Owner: ${cleanText(issue?.owner_component || 'unknown', 180)} (${cleanText(issue?.owner_path || 'unknown', 260)})`,
      `Persistence met: ${persistenceMet ? 'yes' : 'no'}`,
      `Human approval: ${approval ? `yes (${approvedBy || 'approved'})` : 'no'}`,
      '',
      'Summary:',
      cleanText(issue?.summary || '', 500),
      '',
      'Next action:',
      cleanText(issue?.next_action || '', 500),
      '',
      'Acceptance criteria:',
      ...acceptanceCriteria.map((entry) => `- ${entry}`),
      '',
      'Evidence:',
      ...evidenceRows.map((row: any) => {
        return `- (${cleanText(row?.turn_id || 'unknown', 200)}) ${cleanText(row?.snippet || '', 260)}`;
      }),
    ];
    return {
      id: issueId,
      severity,
      confidence,
      issue_fingerprint: cleanText(issue?.issue_fingerprint || '', 80),
      owner_component: cleanText(issue?.owner_component || '', 180),
      owner_path: cleanText(issue?.owner_path || '', 260),
      persistence_threshold_met: persistenceMet,
      approved_by: approvedBy || null,
      ready_to_file: readyToFile,
      blockers,
      title,
      body: bodyLines.join('\n'),
      acceptance_criteria: acceptanceCriteria,
      related_agent_ids: relatedAgentIds,
      evidence,
    };
  });

  const readyDrafts = drafts.filter((row) => row.ready_to_file);
  const policyViolations = readyDrafts.filter((row) => {
    const source = issueById.get(row.id) || {};
    return (
      (requirePersistence && !Boolean(source?.persistence_threshold_met))
      || (requireHumanApproval && !approvalById.has(row.id))
      || Number(source?.confidence || 0) < minConfidence
      || !allowSeverities.has(cleanText(source?.severity || '', 20).toLowerCase())
      || severityRank(cleanText(source?.severity || '', 20)) < minSeverityRank
    );
  });

  const checks = [
    { id: 'monitor_present', ok: fs.existsSync(monitorAbs), detail: args.monitorPath },
    { id: 'policy_present', ok: fs.existsSync(policyAbs), detail: args.policyPath },
    {
      id: 'issue_draft_generation_contract',
      ok: drafts.length === issues.length,
      detail: `issues=${issues.length};drafts=${drafts.length}`,
    },
    {
      id: 'safe_filing_policy_contract',
      ok: policyViolations.length === 0,
      detail: `ready=${readyDrafts.length};violations=${policyViolations.length}`,
    },
  ];

  const report = {
    type: 'eval_issue_filing_guard',
    schema_version: 1,
    generated_at: nowIso,
    ok: checks.every((row) => row.ok),
    checks,
    policy: {
      min_severity: minSeverity,
      min_confidence: minConfidence,
      require_persistence_threshold_met: requirePersistence,
      require_human_approval: requireHumanApproval,
      allow_severities: Array.from(allowSeverities.values()),
    },
    summary: {
      issue_count: issues.length,
      draft_count: drafts.length,
      ready_to_file_count: readyDrafts.length,
      blocked_count: Math.max(0, drafts.length - readyDrafts.length),
    },
    issue_drafts: drafts,
    sources: {
      monitor: args.monitorPath,
      policy: args.policyPath,
      approvals: args.approvalsPath,
    },
  };

  writeJsonArtifact(outLatestAbs, report);
  writeJsonArtifact(draftsStateAbs, {
    type: 'eval_issue_draft_payloads',
    generated_at: nowIso,
    ready_to_file_count: readyDrafts.length,
    issue_drafts: drafts,
  });
  writeTextArtifact(markdownAbs, renderMarkdown(report));
  return emitStructuredResult(report, {
    outPath: path.resolve(root, args.outPath),
    strict: args.strict,
    ok: report.ok,
  });
}

process.exit(run(process.argv.slice(2)));
