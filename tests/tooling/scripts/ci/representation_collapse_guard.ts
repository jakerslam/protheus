#!/usr/bin/env node
/* eslint-disable no-console */
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/representation_collapse_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/REPRESENTATION_COLLAPSE_GUARD_CURRENT.md';
const DEFAULT_POLICY = 'tests/tooling/config/representation_collapse_guard_policy.json';
const ENTRYPOINT = 'client/runtime/lib/ts_entrypoint.ts';
const REPORT_SCRIPT = 'tests/tooling/scripts/ci/representation_collapse_report.ts';
const CURRENT_REPORT = 'core/local/artifacts/representation_collapse_guard_source_report.json';
const CURRENT_MARKDOWN = 'local/workspace/reports/REPRESENTATION_COLLAPSE_GUARD_SOURCE_REPORT.md';
const CONTROLLED_REPORT = 'core/local/artifacts/representation_collapse_guard_controlled_report.json';
const CONTROLLED_MARKDOWN = 'local/workspace/reports/REPRESENTATION_COLLAPSE_GUARD_CONTROLLED_REPORT.md';

type Risk = {
  kind: string;
  severity: 'info' | 'warning' | 'high';
  entity: string;
  path?: string;
  detail: string;
};

type RepresentationReport = {
  ok: boolean;
  type: string;
  summary: {
    risk_count: number;
    high_risk_count: number;
    warning_risk_count: number;
    info_risk_count: number;
  };
  risks: Risk[];
};

type GuardPolicy = {
  version: number;
  current_risk_budgets: Record<string, number>;
  controlled_failure_kinds: string[];
};

type BudgetFailure = {
  kind: string;
  count: number;
  budget: number;
  reason: 'unbudgeted_risk_kind' | 'risk_budget_exceeded';
};

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600),
    policyPath: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY, 600),
    includeControlledViolation: parseBool(readFlag(argv, 'include-controlled-violation'), false),
  };
}

function readJson<T>(relPath: string): T {
  return JSON.parse(fs.readFileSync(abs(relPath), 'utf8')) as T;
}

function runReport(outJson: string, outMarkdown: string, includeControlledViolation: boolean): RepresentationReport {
  const args = [
    ENTRYPOINT,
    REPORT_SCRIPT,
    '--strict=0',
    `--out-json=${outJson}`,
    `--out-markdown=${outMarkdown}`,
  ];
  if (includeControlledViolation) args.push('--include-controlled-violation=1');
  const result = spawnSync(process.execPath, args, {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (result.status !== 0) {
    throw new Error(`representation collapse report failed: ${result.stderr || result.stdout}`);
  }
  return readJson<RepresentationReport>(outJson);
}

function countRisks(report: RepresentationReport): Record<string, number> {
  const counts: Record<string, number> = {};
  for (const risk of report.risks) {
    counts[risk.kind] = (counts[risk.kind] || 0) + 1;
  }
  return counts;
}

function budgetFailures(report: RepresentationReport, policy: GuardPolicy): BudgetFailure[] {
  const counts = countRisks(report);
  const failures: BudgetFailure[] = [];
  for (const [kind, count] of Object.entries(counts).sort(([left], [right]) => left.localeCompare(right))) {
    const budget = policy.current_risk_budgets[kind];
    if (budget === undefined) {
      failures.push({ kind, count, budget: 0, reason: 'unbudgeted_risk_kind' });
    } else if (count > budget) {
      failures.push({ kind, count, budget, reason: 'risk_budget_exceeded' });
    }
  }
  return failures;
}

function renderMarkdown(result: ReturnType<typeof buildGuardResult>): string {
  const lines = [
    '# Representation Collapse Guard',
    '',
    `Status: ${result.ok ? 'pass' : 'fail'}`,
    `Policy: \`${result.policy_path}\``,
    '',
    '## Current report',
    '',
    `Risks: ${result.current_summary.risk_count} (high=${result.current_summary.high_risk_count}, warning=${result.current_summary.warning_risk_count}, info=${result.current_summary.info_risk_count})`,
    '',
    '### Current budget failures',
  ];
  if (result.current_budget_failures.length === 0) {
    lines.push('', 'None.');
  } else {
    for (const failure of result.current_budget_failures) {
      lines.push(`- ${failure.kind}: ${failure.count} > ${failure.budget} (${failure.reason})`);
    }
  }
  lines.push('', '## Controlled negative proof', '');
  lines.push(`Controlled failure kinds present: ${result.controlled_failure_kinds_present.join(', ') || 'none'}`);
  lines.push(`Controlled negative rejected: ${result.controlled_negative_rejected}`);
  if (result.missing_controlled_failure_kinds.length > 0) {
    lines.push(`Missing controlled failure kinds: ${result.missing_controlled_failure_kinds.join(', ')}`);
  }
  lines.push('', '## Artifacts', '');
  for (const artifact of result.artifact_paths) {
    lines.push(`- \`${artifact}\``);
  }
  return `${lines.join('\n')}\n`;
}

function buildGuardResult(args: ReturnType<typeof parseArgs>) {
  const policy = readJson<GuardPolicy>(args.policyPath);
  const current = runReport(CURRENT_REPORT, CURRENT_MARKDOWN, args.includeControlledViolation);
  const controlled = runReport(CONTROLLED_REPORT, CONTROLLED_MARKDOWN, true);
  const currentBudgetFailures = budgetFailures(current, policy);
  const controlledBudgetFailures = budgetFailures(controlled, policy);
  const controlledCounts = countRisks(controlled);
  const controlledFailureKindsPresent = policy.controlled_failure_kinds.filter((kind) => (controlledCounts[kind] || 0) > 0);
  const missingControlledFailureKinds = policy.controlled_failure_kinds.filter((kind) => !controlledFailureKindsPresent.includes(kind));
  const controlledNegativeRejected = controlledBudgetFailures.some((failure) => policy.controlled_failure_kinds.includes(failure.kind));
  const ok = currentBudgetFailures.length === 0 && missingControlledFailureKinds.length === 0 && controlledNegativeRejected;
  return {
    ok,
    type: 'representation_collapse_guard',
    policy_path: args.policyPath,
    current_summary: current.summary,
    controlled_summary: controlled.summary,
    current_risk_counts: countRisks(current),
    current_budget_failures: currentBudgetFailures,
    controlled_budget_failures: controlledBudgetFailures,
    controlled_failure_kinds_present: controlledFailureKindsPresent,
    missing_controlled_failure_kinds: missingControlledFailureKinds,
    controlled_negative_rejected: controlledNegativeRejected,
    artifact_paths: [args.outJson, args.outMarkdown, CURRENT_REPORT, CURRENT_MARKDOWN, CONTROLLED_REPORT, CONTROLLED_MARKDOWN],
  };
}

const args = parseArgs(process.argv.slice(2));
const result = buildGuardResult(args);
writeTextArtifact(args.outMarkdown, renderMarkdown(result));
process.exitCode = emitStructuredResult(result, {
  outPath: args.outJson,
  strict: args.strict,
  ok: result.ok,
});
