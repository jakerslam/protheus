#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type DriftViolation = {
  check_id?: string;
  boundary_id?: string;
  file?: string;
  detail?: string;
  owner_layer?: string;
  required_policy_test?: string;
  runtime_contract_fix_required?: boolean;
  suggested_contract_fix?: string;
};

type PolicyFailure = {
  id?: string;
  detail?: string;
};

type OwnershipDriftGuardPayload = {
  ok?: boolean;
  policy_failures?: PolicyFailure[];
  violations?: DriftViolation[];
};

type DuplicateLogicCandidate = {
  signature: string;
  check_id: string;
  detail: string;
  occurrence_count: number;
  files: string[];
  boundary_ids: string[];
  severity: 'low' | 'medium' | 'high';
};

type MultiResponsibilityCandidate = {
  file: string;
  check_ids: string[];
  boundary_ids: string[];
  violation_count: number;
  severity: 'medium' | 'high';
};

type Args = {
  strict: boolean;
  sourcePath: string;
  outJsonPath: string;
  outMarkdownPath: string;
  reportAliasPath: string;
  highSeverityThreshold: number;
  severityBudgetLow: number;
  severityBudgetMedium: number;
  severityBudgetHigh: number;
  unresolvedHighThreshold: number;
};

const ROOT = process.cwd();
const DEFAULT_SOURCE_PATH = 'core/local/artifacts/ownership_drift_guard_current.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/ownership_drift_weekly_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/OWNERSHIP_DRIFT_WEEKLY_CURRENT.md';
const DEFAULT_REPORT_ALIAS = 'local/workspace/reports/ownership_drift_report.md';

function rel(p: string): string {
  return path.relative(ROOT, p).replace(/\\/g, '/');
}

function parseArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, {
    strict: false,
    out: DEFAULT_OUT_JSON,
  });
  const thresholdRaw = cleanText(readFlag(argv, 'high-severity-threshold') || '0', 20);
  const thresholdParsed = Number.parseInt(thresholdRaw, 10);
  const lowBudgetRaw = cleanText(readFlag(argv, 'severity-budget-low') || '999999', 20);
  const mediumBudgetRaw = cleanText(readFlag(argv, 'severity-budget-medium') || '999999', 20);
  const highBudgetRaw = cleanText(readFlag(argv, 'severity-budget-high') || thresholdRaw || '0', 20);
  const unresolvedHighRaw = cleanText(readFlag(argv, 'unresolved-high-threshold') || thresholdRaw || '0', 20);
  const lowBudgetParsed = Number.parseInt(lowBudgetRaw, 10);
  const mediumBudgetParsed = Number.parseInt(mediumBudgetRaw, 10);
  const highBudgetParsed = Number.parseInt(highBudgetRaw, 10);
  const unresolvedHighParsed = Number.parseInt(unresolvedHighRaw, 10);
  return {
    strict: common.strict,
    sourcePath: cleanText(readFlag(argv, 'source') || DEFAULT_SOURCE_PATH, 400),
    outJsonPath: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
    reportAliasPath: cleanText(readFlag(argv, 'report-alias') || DEFAULT_REPORT_ALIAS, 400),
    highSeverityThreshold:
      Number.isFinite(thresholdParsed) && thresholdParsed >= 0 ? thresholdParsed : 0,
    severityBudgetLow:
      Number.isFinite(lowBudgetParsed) && lowBudgetParsed >= 0 ? lowBudgetParsed : 999999,
    severityBudgetMedium:
      Number.isFinite(mediumBudgetParsed) && mediumBudgetParsed >= 0 ? mediumBudgetParsed : 999999,
    severityBudgetHigh:
      Number.isFinite(highBudgetParsed) && highBudgetParsed >= 0 ? highBudgetParsed : 0,
    unresolvedHighThreshold:
      Number.isFinite(unresolvedHighParsed) && unresolvedHighParsed >= 0 ? unresolvedHighParsed : 0,
  };
}

function asArray<T = any>(value: unknown): T[] {
  return Array.isArray(value) ? (value as T[]) : [];
}

function computeDuplicateLogicCandidates(violations: DriftViolation[]): DuplicateLogicCandidate[] {
  const groups = new Map<
    string,
    {
      checkId: string;
      detail: string;
      files: Set<string>;
      boundaries: Set<string>;
      count: number;
    }
  >();

  for (const row of violations) {
    const checkId = cleanText(row.check_id || 'unknown', 120) || 'unknown';
    const detail = cleanText(row.detail || 'unspecified', 260) || 'unspecified';
    const file = cleanText(row.file || '', 500);
    const boundary = cleanText(row.boundary_id || '', 160);
    const signature = `${checkId}::${detail}`;
    const group = groups.get(signature) || {
      checkId,
      detail,
      files: new Set<string>(),
      boundaries: new Set<string>(),
      count: 0,
    };
    if (file) group.files.add(file);
    if (boundary) group.boundaries.add(boundary);
    group.count += 1;
    groups.set(signature, group);
  }

  const out: DuplicateLogicCandidate[] = [];
  for (const [signature, group] of groups) {
    if (group.files.size < 2) continue;
    const fileCount = group.files.size;
    const severity: 'low' | 'medium' | 'high' =
      fileCount >= 4 ? 'high' : fileCount >= 3 ? 'medium' : 'low';
    out.push({
      signature,
      check_id: group.checkId,
      detail: group.detail,
      occurrence_count: group.count,
      files: Array.from(group.files).sort((a, b) => a.localeCompare(b)),
      boundary_ids: Array.from(group.boundaries).sort((a, b) => a.localeCompare(b)),
      severity,
    });
  }
  out.sort((a, b) => b.occurrence_count - a.occurrence_count || a.signature.localeCompare(b.signature));
  return out;
}

function computeMultiResponsibilityCandidates(
  violations: DriftViolation[],
): MultiResponsibilityCandidate[] {
  const groups = new Map<
    string,
    {
      checkIds: Set<string>;
      boundaries: Set<string>;
      count: number;
    }
  >();

  for (const row of violations) {
    const file = cleanText(row.file || '', 500);
    if (!file) continue;
    const checkId = cleanText(row.check_id || 'unknown', 120) || 'unknown';
    const boundary = cleanText(row.boundary_id || 'unknown', 160) || 'unknown';
    const group = groups.get(file) || {
      checkIds: new Set<string>(),
      boundaries: new Set<string>(),
      count: 0,
    };
    group.checkIds.add(checkId);
    group.boundaries.add(boundary);
    group.count += 1;
    groups.set(file, group);
  }

  const out: MultiResponsibilityCandidate[] = [];
  for (const [file, group] of groups) {
    if (group.count < 2) continue;
    const checkIds = Array.from(group.checkIds).sort((a, b) => a.localeCompare(b));
    const boundaries = Array.from(group.boundaries).sort((a, b) => a.localeCompare(b));
    if (checkIds.length < 2 && boundaries.length < 2) continue;
    const severity: 'medium' | 'high' = checkIds.length >= 2 ? 'high' : 'medium';
    out.push({
      file,
      check_ids: checkIds,
      boundary_ids: boundaries,
      violation_count: group.count,
      severity,
    });
  }
  out.sort((a, b) => b.violation_count - a.violation_count || a.file.localeCompare(b.file));
  return out;
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Ownership Drift Weekly Report');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Source: ${payload.inputs.source_path}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push('');
  lines.push(`- Policy failures: ${payload.summary.policy_failure_count}`);
  lines.push(`- Drift violations: ${payload.summary.violation_count}`);
  lines.push(`- Duplicate-logic candidates: ${payload.summary.duplicate_logic_candidate_count}`);
  lines.push(
    `- Multi-responsibility candidates: ${payload.summary.multi_responsibility_candidate_count}`,
  );
  lines.push(`- Runtime/API contract fix candidates: ${payload.summary.contract_fix_candidate_count}`);
  lines.push(`- High-severity findings: ${payload.summary.high_severity_count}`);
  lines.push(
    `- High-severity threshold: ${payload.summary.high_severity_threshold} (pass when count <= threshold)`,
  );
  lines.push(
    `- Severity budgets (low/medium/high): ${payload.summary.severity_budget_low}/${payload.summary.severity_budget_medium}/${payload.summary.severity_budget_high}`,
  );
  lines.push(
    `- Severity counts (low/medium/high): ${payload.summary.severity_count_low}/${payload.summary.severity_count_medium}/${payload.summary.severity_count_high}`,
  );
  lines.push(
    `- Unresolved high-severity threshold: ${payload.summary.unresolved_high_threshold} (actual=${payload.summary.unresolved_high_count})`,
  );
  lines.push('');
  lines.push('## High-Severity Findings');
  lines.push('');
  if (payload.high_severity_findings.length === 0) {
    lines.push('- none');
  } else {
    for (const row of payload.high_severity_findings) {
      lines.push(`- ${row.id}: ${row.detail}`);
    }
  }
  lines.push('');
  lines.push('## Duplicate-Logic Candidates');
  lines.push('');
  lines.push('| Severity | Signature | Files | Count |');
  lines.push('| --- | --- | --- | --- |');
  if (payload.duplicate_logic_candidates.length === 0) {
    lines.push('| (none) | - | - | - |');
  } else {
    for (const row of payload.duplicate_logic_candidates.slice(0, 150)) {
      lines.push(
        `| ${row.severity} | ${String(row.signature).slice(0, 120)} | ${row.files.length} | ${row.occurrence_count} |`,
      );
    }
  }
  lines.push('');
  lines.push('## Multi-Responsibility Placement Candidates');
  lines.push('');
  lines.push('| Severity | File | Checks | Boundaries | Violations |');
  lines.push('| --- | --- | --- | --- | --- |');
  if (payload.multi_responsibility_candidates.length === 0) {
    lines.push('| (none) | - | - | - | - |');
  } else {
    for (const row of payload.multi_responsibility_candidates.slice(0, 150)) {
      lines.push(
        `| ${row.severity} | ${String(row.file).slice(0, 120)} | ${row.check_ids.length} | ${row.boundary_ids.length} | ${row.violation_count} |`,
      );
    }
  }
  lines.push('');
  lines.push('## Runtime/API Contract Fix Candidates');
  lines.push('');
  lines.push('| Owner | Policy Test | File | Boundary | Suggested Fix |');
  lines.push('| --- | --- | --- | --- | --- |');
  if (!Array.isArray(payload.contract_fix_candidates) || payload.contract_fix_candidates.length === 0) {
    lines.push('| (none) | - | - | - | - |');
  } else {
    for (const row of payload.contract_fix_candidates.slice(0, 150)) {
      lines.push(
        `| ${String(row.owner_layer || 'unknown')} | ${String(
          row.required_policy_test || 'placement-test:unknown',
        )} | ${String(row.file || '').slice(0, 120)} | ${String(
          row.boundary_id || '',
        ).slice(0, 120)} | ${String(row.suggested_contract_fix || '').slice(0, 160)} |`,
      );
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function writeReportMarkdown(args: Args, payload: any): void {
  const markdown = toMarkdown(payload);
  const paths = Array.from(new Set([args.outMarkdownPath, args.reportAliasPath].filter(Boolean)));
  for (const row of paths) {
    writeTextArtifact(path.resolve(ROOT, row), markdown);
  }
}

function main(): number {
  const args = parseArgs(process.argv.slice(2));
  const sourceAbsPath = path.resolve(ROOT, args.sourcePath);
  let source: OwnershipDriftGuardPayload;

  try {
    source = JSON.parse(fs.readFileSync(sourceAbsPath, 'utf8')) as OwnershipDriftGuardPayload;
  } catch (err) {
    const payload = {
      ok: false,
      type: 'ownership_drift_weekly_report',
      generated_at: new Date().toISOString(),
      revision: currentRevision(ROOT),
      inputs: {
        strict: args.strict,
        source_path: args.sourcePath,
        out_markdown: args.outMarkdownPath,
        report_alias: args.reportAliasPath,
      },
      summary: {
        source_read_error: true,
        high_severity_count: 1,
        high_severity_threshold: args.highSeverityThreshold,
      },
      failures: [
        {
          id: 'ownership_drift_weekly_source_read_failed',
          detail: cleanText(String(err), 500),
        },
      ],
    };
    writeReportMarkdown(args, payload);
    return emitStructuredResult(payload, {
      outPath: path.resolve(ROOT, args.outJsonPath),
      strict: args.strict,
      ok: false,
    });
  }

  const policyFailures = asArray<PolicyFailure>(source.policy_failures).map((row) => ({
    id: cleanText(row.id || 'policy_failure', 200) || 'policy_failure',
    detail: cleanText(row.detail || 'unspecified', 400) || 'unspecified',
  }));
  const violations = asArray<DriftViolation>(source.violations).map((row) => ({
    check_id: cleanText(row.check_id || 'unknown', 120) || 'unknown',
    boundary_id: cleanText(row.boundary_id || 'unknown', 160) || 'unknown',
    file: cleanText(row.file || 'unknown', 500) || 'unknown',
    detail: cleanText(row.detail || 'unspecified', 260) || 'unspecified',
    owner_layer: cleanText(row.owner_layer || 'unknown', 80) || 'unknown',
    required_policy_test:
      cleanText(row.required_policy_test || 'placement-test:unknown', 120)
      || 'placement-test:unknown',
    runtime_contract_fix_required: row.runtime_contract_fix_required === true,
    suggested_contract_fix:
      cleanText(row.suggested_contract_fix || 'Add an explicit owner-layer contract.', 400)
      || 'Add an explicit owner-layer contract.',
  }));

  const duplicateLogicCandidates = computeDuplicateLogicCandidates(violations);
  const multiResponsibilityCandidates = computeMultiResponsibilityCandidates(violations);
  const contractFixCandidates = violations.filter(
    (row) => row.runtime_contract_fix_required === true,
  );

  const highSeverityFindings: Array<{ id: string; detail: string; severity: 'high' }> = [];
  const severityFindings: Array<{ id: string; detail: string; severity: 'low' | 'medium' | 'high' }> = [];
  for (const row of policyFailures) {
    severityFindings.push({
      id: `policy:${row.id}`,
      detail: row.detail,
      severity: 'high',
    });
    highSeverityFindings.push({
      id: `policy:${row.id}`,
      detail: row.detail,
      severity: 'high',
    });
  }
  for (const row of duplicateLogicCandidates) {
    severityFindings.push({
      id: `duplicate_logic:${row.signature}`,
      detail: `${row.signature};files=${row.files.length};count=${row.occurrence_count}`,
      severity: row.severity,
    });
  }
  for (const row of multiResponsibilityCandidates) {
    severityFindings.push({
      id: `multi_responsibility:${row.file}`,
      detail: `${row.file};checks=${row.check_ids.join(',')};violations=${row.violation_count}`,
      severity: row.severity,
    });
  }
  for (const row of duplicateLogicCandidates) {
    if (row.severity !== 'high') continue;
    highSeverityFindings.push({
      id: 'duplicate_logic_high',
      detail: `${row.signature};files=${row.files.length};count=${row.occurrence_count}`,
      severity: 'high',
    });
  }
  for (const row of multiResponsibilityCandidates) {
    if (row.severity !== 'high') continue;
    highSeverityFindings.push({
      id: 'multi_responsibility_high',
      detail: `${row.file};checks=${row.check_ids.join(',')};violations=${row.violation_count}`,
      severity: 'high',
    });
  }

  const highSeverityCount = highSeverityFindings.length;
  const thresholdPass = highSeverityCount <= args.highSeverityThreshold;
  const severityCountLow = severityFindings.filter((row) => row.severity === 'low').length;
  const severityCountMedium = severityFindings.filter((row) => row.severity === 'medium').length;
  const severityCountHigh = severityFindings.filter((row) => row.severity === 'high').length;
  const unresolvedHighCount = severityCountHigh;
  const severityBudgetPass =
    severityCountLow <= args.severityBudgetLow
    && severityCountMedium <= args.severityBudgetMedium
    && severityCountHigh <= args.severityBudgetHigh;
  const unresolvedHighPass = unresolvedHighCount <= args.unresolvedHighThreshold;
  const overallPass = thresholdPass && severityBudgetPass && unresolvedHighPass;

  const failures: Array<{ id: string; detail: string }> = [];
  if (!thresholdPass) {
    for (const row of highSeverityFindings) {
      failures.push({
        id: 'ownership_drift_high_severity_threshold_exceeded',
        detail: `${row.id}:${row.detail}`,
      });
    }
  }
  if (severityCountLow > args.severityBudgetLow) {
    failures.push({
      id: 'ownership_drift_severity_budget_low_exceeded',
      detail: `actual=${severityCountLow};budget=${args.severityBudgetLow}`,
    });
  }
  if (severityCountMedium > args.severityBudgetMedium) {
    failures.push({
      id: 'ownership_drift_severity_budget_medium_exceeded',
      detail: `actual=${severityCountMedium};budget=${args.severityBudgetMedium}`,
    });
  }
  if (severityCountHigh > args.severityBudgetHigh) {
    failures.push({
      id: 'ownership_drift_severity_budget_high_exceeded',
      detail: `actual=${severityCountHigh};budget=${args.severityBudgetHigh}`,
    });
  }
  if (!unresolvedHighPass) {
    failures.push({
      id: 'ownership_drift_unresolved_high_threshold_exceeded',
      detail: `actual=${unresolvedHighCount};threshold=${args.unresolvedHighThreshold}`,
    });
  }

  const payload = {
    ok: overallPass,
    type: 'ownership_drift_weekly_report',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      source_path: rel(sourceAbsPath),
      out_json: args.outJsonPath,
      out_markdown: args.outMarkdownPath,
      report_alias: args.reportAliasPath,
      high_severity_threshold: args.highSeverityThreshold,
      severity_budget_low: args.severityBudgetLow,
      severity_budget_medium: args.severityBudgetMedium,
      severity_budget_high: args.severityBudgetHigh,
      unresolved_high_threshold: args.unresolvedHighThreshold,
      source_guard_ok: Boolean(source.ok),
    },
    summary: {
      policy_failure_count: policyFailures.length,
      violation_count: violations.length,
      duplicate_logic_candidate_count: duplicateLogicCandidates.length,
      multi_responsibility_candidate_count: multiResponsibilityCandidates.length,
      contract_fix_candidate_count: contractFixCandidates.length,
      high_severity_count: highSeverityCount,
      high_severity_threshold: args.highSeverityThreshold,
      severity_count_low: severityCountLow,
      severity_count_medium: severityCountMedium,
      severity_count_high: severityCountHigh,
      severity_budget_low: args.severityBudgetLow,
      severity_budget_medium: args.severityBudgetMedium,
      severity_budget_high: args.severityBudgetHigh,
      unresolved_high_count: unresolvedHighCount,
      unresolved_high_threshold: args.unresolvedHighThreshold,
      threshold_pass: thresholdPass,
      severity_budget_pass: severityBudgetPass,
      unresolved_high_pass: unresolvedHighPass,
    },
    duplicate_logic_candidates: duplicateLogicCandidates,
    multi_responsibility_candidates: multiResponsibilityCandidates,
    contract_fix_candidates: contractFixCandidates,
    severity_findings: severityFindings,
    high_severity_findings: highSeverityFindings,
    failures,
  };

  writeReportMarkdown(args, payload);
  return emitStructuredResult(payload, {
    outPath: path.resolve(ROOT, args.outJsonPath),
    strict: args.strict,
    ok: payload.ok,
  });
}

const exitCode = main();
if (exitCode !== 0) process.exit(exitCode);
