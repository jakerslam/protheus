#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_POLICY = 'validation/tests/contracts/tooling_harness_only_policy.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/tooling_harness_only_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/TOOLING_HARNESS_ONLY_GUARD_CURRENT.md';
const GATE_ID = 'ops:tooling:harness-only:guard';

type Violation = { kind: string; path: string; detail: string };

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function readJson(relPath: string): any {
  return JSON.parse(fs.readFileSync(abs(relPath), 'utf8'));
}

function listFiles(relDir: string): string[] {
  const dir = abs(relDir);
  if (!fs.existsSync(dir)) return [];
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const relPath = path.posix.join(relDir, entry.name);
    if (entry.isDirectory()) return listFiles(relPath);
    return [relPath];
  });
}

function push(violations: Violation[], kind: string, pathRel: string, detail: string): void {
  violations.push({ kind, path: pathRel, detail });
}

function daysUntil(dateText: string): number | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(String(dateText || ''))) return null;
  const then = new Date(`${dateText}T00:00:00Z`).getTime();
  if (!Number.isFinite(then)) return null;
  const now = new Date();
  const today = Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate());
  return Math.floor((then - today) / 86_400_000);
}

function truthyFlag(argv: string[], name: string): boolean {
  const value = readFlag(argv, name);
  return value === '1' || value === 'true' || value === 'yes';
}

function isHarnessAllowed(pathRel: string, policy: any): boolean {
  return (policy.harness_only_prefixes || []).some((prefix: string) => pathRel === prefix || pathRel.startsWith(prefix));
}

function isDefinitionShaped(pathRel: string, policy: any): boolean {
  const lower = path.basename(pathRel).toLowerCase();
  return (policy.definition_shape_patterns || []).some((token: string) => lower.includes(String(token).toLowerCase()));
}

function validatePolicyShape(policyPath: string, policy: any, violations: Violation[]): void {
  if (!policyPath.startsWith('validation/tests/')) {
    push(violations, 'policy_not_validation_owned', policyPath, 'Tooling harness-only policy must live under validation/tests.');
  }
  if (policy.owner !== 'validation/tests') push(violations, 'wrong_policy_owner', policyPath, 'Policy owner must be validation/tests.');
  if (policy.status !== 'enforced') push(violations, 'policy_not_enforced', policyPath, 'Policy status must be enforced.');
  for (const prefix of ['tests/tooling/scripts/', 'tests/tooling/config/tooling_gate_registry.json', 'tests/tooling/config/verify_profiles.json']) {
    if (!(policy.harness_only_prefixes || []).includes(prefix)) {
      push(violations, 'missing_harness_only_prefix', policyPath, `Missing harness-only prefix ${prefix}.`);
    }
  }
  for (const prefix of ['validation/tests/', 'validation/evals/', 'validation/benchmarks/', 'validation/conformance/', 'validation/regression/', 'observability/', 'governance/']) {
    if (!(policy.canonical_definition_prefixes || []).includes(prefix)) {
      push(violations, 'missing_canonical_definition_prefix', policyPath, `Missing canonical definition prefix ${prefix}.`);
    }
  }
}

function validateDebtRows(policy: any, violations: Violation[]): Set<string> {
  const declared = new Set<string>();
  const canonical = policy.canonical_definition_prefixes || [];
  for (const row of policy.declared_transition_debt || []) {
    const pathRel = String(row.path || '');
    declared.add(pathRel);
    if (!pathRel.startsWith('tests/tooling/')) push(violations, 'debt_path_not_tooling', pathRel || DEFAULT_POLICY, 'Transition debt path must be under tests/tooling.');
    if (!fs.existsSync(abs(pathRel))) push(violations, 'declared_debt_missing_file', pathRel, 'Declared tooling definition debt file does not exist.');
    if (!row.owner || !row.reason || !row.target_domain || !row.target_prefix) {
      push(violations, 'incomplete_debt_row', pathRel || DEFAULT_POLICY, 'Debt row needs owner, reason, target_domain, and target_prefix.');
    }
    if (!canonical.some((prefix: string) => String(row.target_prefix || '').startsWith(prefix))) {
      push(violations, 'debt_target_not_canonical', pathRel || DEFAULT_POLICY, `Target prefix is not a canonical Assurance definition prefix: ${row.target_prefix || 'missing'}.`);
    }
    const days = daysUntil(row.expires);
    if (days === null) push(violations, 'invalid_debt_expiry', pathRel || DEFAULT_POLICY, 'Debt expiry must be YYYY-MM-DD.');
    else if (days < 0) push(violations, 'expired_debt_row', pathRel || DEFAULT_POLICY, `Debt row expired ${Math.abs(days)} day(s) ago.`);
  }
  return declared;
}

function validateToolingFiles(policy: any, declaredDebt: Set<string>, violations: Violation[], includeControlledViolation: boolean): any {
  const toolingFiles = listFiles('tests/tooling').filter((file) => /\.(json|jsonl|ya?ml|toml|md)$/.test(file));
  if (includeControlledViolation) toolingFiles.push('tests/tooling/config/undocumented_eval_policy.json');
  const candidates = toolingFiles.filter((file) => !isHarnessAllowed(file, policy) && isDefinitionShaped(file, policy));
  const undeclared = candidates.filter((file) => !declaredDebt.has(file));
  for (const file of undeclared) {
    push(violations, 'undeclared_tooling_definition', file, 'Definition-shaped tooling file must move to Validation/Observability/Governance or be declared as time-bounded transition debt.');
  }
  return {
    scanned_files: toolingFiles.length,
    definition_shaped_candidates: candidates.length,
    declared_transition_debt_count: declaredDebt.size,
    undeclared_definition_count: undeclared.length,
    undeclared_definitions: undeclared,
  };
}

function validateWiring(violations: Violation[]): void {
  const pkg = readJson('package.json');
  const gates = readJson('tests/tooling/config/tooling_gate_registry.json').gates || {};
  if (!String(pkg.scripts?.[GATE_ID] || '').includes('tooling_harness_only_guard.ts')) {
    push(violations, 'missing_package_script', 'package.json', `${GATE_ID} must execute tooling_harness_only_guard.ts.`);
  }
  if (!gates[GATE_ID]) {
    push(violations, 'missing_tooling_registry_row', 'tests/tooling/config/tooling_gate_registry.json', `${GATE_ID} must be registered.`);
  }
}

function renderMarkdown(report: any): string {
  const lines = [
    '# Tooling Harness-Only Guard',
    '',
    `ok: ${report.ok}`,
    `revision: ${report.revision}`,
    `policy: ${report.policy_path}`,
    `controlled_violation: ${report.controlled_violation}`,
    `violations: ${report.violations.length}`,
    '',
    '## Summary',
    `- scanned files: ${report.summary.scanned_files}`,
    `- definition-shaped candidates: ${report.summary.definition_shaped_candidates}`,
    `- declared transition debt: ${report.summary.declared_transition_debt_count}`,
    `- undeclared definitions: ${report.summary.undeclared_definition_count}`,
    '',
    '## Violations',
  ];
  if (report.violations.length === 0) lines.push('- none');
  for (const violation of report.violations) lines.push(`- ${violation.kind} at \`${violation.path}\`: ${violation.detail}`);
  return `${lines.join('\n')}\n`;
}

const argv = process.argv.slice(2);
const args = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
const policyPath = cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY, 600);
const outJson = cleanText(readFlag(argv, 'out-json') || args.out || DEFAULT_OUT_JSON, 600);
const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);
const controlledViolation = truthyFlag(argv, 'include-controlled-violation');
const policy = readJson(policyPath);
const violations: Violation[] = [];
validatePolicyShape(policyPath, policy, violations);
const declaredDebt = validateDebtRows(policy, violations);
const summary = validateToolingFiles(policy, declaredDebt, violations, controlledViolation);
validateWiring(violations);
const report = {
  ok: violations.length === 0,
  type: 'tooling_harness_only_guard',
  revision: currentRevision(ROOT),
  policy_path: policyPath,
  controlled_violation: controlledViolation,
  summary,
  violations,
};
writeTextArtifact(outMarkdown, renderMarkdown(report));
process.exitCode = emitStructuredResult(report, { outPath: outJson, strict: args.strict, ok: report.ok });
