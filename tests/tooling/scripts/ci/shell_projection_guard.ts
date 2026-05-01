#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision, trackedFiles } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_POLICY = 'tests/tooling/config/shell_projection_guard_policy.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_projection_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_PROJECTION_GUARD_CURRENT.md';

type Pattern = {
  id: string;
  description: string;
  regex: string;
};

type Allowance = {
  path: string;
  max: number;
  expires: string;
  reason: string;
  replacement_plan: string;
};

type Policy = {
  version?: string;
  policy_doc_path: string;
  policy_doc_required_tokens?: string[];
  scan_roots?: string[];
  scan_extensions?: string[];
  ignore_path_contains?: string[];
  forbidden_patterns?: Pattern[];
  allowed_legacy_occurrences?: Record<string, Allowance[]>;
};

type Args = {
  strict: boolean;
  policyPath: string;
  scanRoots: string[];
  outJson: string;
  outMarkdown: string;
  includeControlledViolation: boolean;
};

type Hit = {
  pattern_id: string;
  description: string;
  path: string;
  matches: number;
  allowed: boolean;
  allowance?: Allowance;
};

type Violation = {
  kind: string;
  pattern_id?: string;
  path?: string;
  detail: string;
};

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function readJson<T>(relPath: string): T {
  return JSON.parse(fs.readFileSync(abs(relPath), 'utf8')) as T;
}

function readText(relPath: string): string {
  return fs.readFileSync(abs(relPath), 'utf8');
}

function exists(relPath: string): boolean {
  return fs.existsSync(abs(relPath));
}

function parseList(raw: string | undefined): string[] {
  return cleanText(raw || '', 3000)
    .split(',')
    .map((row) => cleanText(row, 600))
    .filter(Boolean);
}

function parseArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    policyPath: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY, 600),
    scanRoots: parseList(readFlag(argv, 'scan-roots')),
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600),
    includeControlledViolation: parseBool(readFlag(argv, 'include-controlled-violation'), false),
  };
}

function unique<T>(rows: T[]): T[] {
  return Array.from(new Set(rows));
}

function expandScanFiles(policy: Policy, overrideRoots: string[]): string[] {
  const roots = overrideRoots.length ? overrideRoots : (policy.scan_roots || []);
  const extensions = new Set(policy.scan_extensions || ['.ts', '.tsx', '.html', '.css']);
  const ignored = policy.ignore_path_contains || [];
  const tracked = trackedFiles(ROOT);
  const out: string[] = [];
  for (const root of roots) {
    if (exists(root) && fs.statSync(abs(root)).isFile()) {
      out.push(root);
      continue;
    }
    const prefix = root.endsWith('/') ? root : `${root}/`;
    for (const file of tracked) {
      if (!file.startsWith(prefix)) continue;
      if (!extensions.has(path.extname(file))) continue;
      out.push(file);
    }
  }
  return unique(out)
    .filter((file) => !ignored.some((needle) => needle && file.includes(needle)))
    .sort();
}

function compile(pattern: Pattern, violations: Violation[]): RegExp | null {
  try {
    return new RegExp(pattern.regex, 'g');
  } catch (error) {
    violations.push({
      kind: 'invalid_projection_guard_regex',
      pattern_id: pattern.id,
      detail: `${pattern.description}: ${String(error)}`,
    });
    return null;
  }
}

function countMatches(source: string, regex: RegExp): number {
  let count = 0;
  regex.lastIndex = 0;
  while (regex.exec(source)) {
    count += 1;
    if (regex.lastIndex === 0) break;
  }
  return count;
}

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

function findAllowance(policy: Policy, patternId: string, file: string): Allowance | null {
  const rows = (policy.allowed_legacy_occurrences || {})[patternId] || [];
  return rows.find((row) => row.path === file) || null;
}

function validateAllowance(pattern: Pattern, allowance: Allowance | null, matches: number, violations: Violation[]): boolean {
  if (!allowance) return false;
  const missing = ['reason', 'replacement_plan', 'expires'].filter((key) => !cleanText((allowance as any)[key], 800));
  if (missing.length) {
    violations.push({
      kind: 'invalid_projection_guard_legacy_allowance',
      pattern_id: pattern.id,
      path: allowance.path,
      detail: `Legacy allowance is missing ${missing.join(', ')}.`,
    });
    return false;
  }
  if (allowance.expires < today()) {
    violations.push({
      kind: 'expired_projection_guard_legacy_allowance',
      pattern_id: pattern.id,
      path: allowance.path,
      detail: `Legacy allowance expired at ${allowance.expires}.`,
    });
    return false;
  }
  if (matches > Number(allowance.max || 0)) {
    violations.push({
      kind: 'projection_guard_legacy_allowance_exceeded',
      pattern_id: pattern.id,
      path: allowance.path,
      detail: `Observed ${matches} matches, allowance permits ${allowance.max}.`,
    });
    return false;
  }
  return true;
}

function validatePolicyDoc(policy: Policy, violations: Violation[]): void {
  if (!policy.policy_doc_path || !exists(policy.policy_doc_path)) {
    violations.push({ kind: 'missing_shell_projection_policy_doc', detail: 'Canonical Shell UI Projection policy document is missing.' });
    return;
  }
  const doc = readText(policy.policy_doc_path);
  for (const token of policy.policy_doc_required_tokens || []) {
    if (!doc.includes(token)) {
      violations.push({
        kind: 'shell_projection_policy_doc_missing_token',
        path: policy.policy_doc_path,
        detail: `Missing required policy token: ${token}`,
      });
    }
  }
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell Projection Guard');
  lines.push('');
  lines.push(`- Generated at: ${payload.generated_at}`);
  lines.push(`- Revision: ${payload.revision}`);
  lines.push(`- Pass: ${payload.ok}`);
  lines.push(`- Policy: ${payload.policy_path}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- scanned_files: ${payload.summary.scanned_files}`);
  lines.push(`- forbidden_pattern_count: ${payload.summary.forbidden_pattern_count}`);
  lines.push(`- strict_violations: ${payload.summary.strict_violations}`);
  lines.push(`- known_debt_matches: ${payload.summary.known_debt_matches}`);
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) lines.push('- none');
  for (const violation of payload.violations) {
    lines.push(`- ${violation.kind}: ${violation.pattern_id || '-'} ${violation.path || ''} ${violation.detail}`);
  }
  lines.push('');
  lines.push('## Known Debt');
  if (!payload.known_debt.length) lines.push('- none');
  for (const hit of payload.known_debt) {
    lines.push(`- ${hit.pattern_id}: ${hit.path} matches=${hit.matches} expires=${hit.allowance.expires}`);
  }
  return `${lines.join('\n')}\n`;
}

async function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const policy = readJson<Policy>(args.policyPath);
  const violations: Violation[] = [];
  const hits: Hit[] = [];
  const knownDebt: Hit[] = [];
  validatePolicyDoc(policy, violations);

  const files = expandScanFiles(policy, args.scanRoots);
  const virtualSources: Record<string, string> = {};
  if (args.includeControlledViolation) {
    virtualSources['local/workspace/shadow/controlled-shell-projection-violation.ts'] =
      'const snapshot = { raw: store, root: rootState }; String(tool.result);';
  }

  for (const pattern of policy.forbidden_patterns || []) {
    const regex = compile(pattern, violations);
    if (!regex) continue;
    for (const file of [...files, ...Object.keys(virtualSources)]) {
      const source = virtualSources[file] == null ? readText(file) : virtualSources[file];
      const matches = countMatches(source, regex);
      if (!matches) continue;
      const allowance = findAllowance(policy, pattern.id, file);
      const allowed = validateAllowance(pattern, allowance, matches, violations);
      const hit = { pattern_id: pattern.id, description: pattern.description, path: file, matches, allowed, allowance: allowance || undefined };
      hits.push(hit);
      if (allowed) knownDebt.push(hit);
      else {
        violations.push({
          kind: 'shell_projection_forbidden_pattern',
          pattern_id: pattern.id,
          path: file,
          detail: `${pattern.description} matches=${matches}`,
        });
      }
    }
  }

  const payload = {
    ok: violations.length === 0,
    type: 'shell_projection_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: args.strict,
    policy_path: args.policyPath,
    summary: {
      scanned_files: files.length,
      forbidden_pattern_count: (policy.forbidden_patterns || []).length,
      pattern_hits: hits.length,
      strict_violations: violations.length,
      known_debt_matches: knownDebt.reduce((sum, hit) => sum + hit.matches, 0),
    },
    violations,
    known_debt: knownDebt,
    hits,
  };
  writeTextArtifact(args.outMarkdown, markdown(payload));
  emitStructuredResult(payload, { ok: payload.ok, outPath: args.outJson });
  if (args.strict && !payload.ok) process.exitCode = 1;
}

run().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
