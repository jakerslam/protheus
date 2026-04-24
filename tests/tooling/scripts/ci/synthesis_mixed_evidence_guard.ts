#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult } from '../../lib/result.ts';

type EvidenceSource = 'workspace' | 'web' | 'tool' | 'memory';
type ContradictionStatus = 'resolved' | 'unresolved';

type EvidenceCase = {
  id: string;
  source_type: EvidenceSource;
  claim: string;
};

type ContradictionCase = {
  left: string;
  right: string;
  status: ContradictionStatus;
};

type ExpectedShape = {
  required_sections: string[];
  must_reference_evidence_ids: string[];
  max_dropped_evidence: number;
  requires_clarification_on_unresolved_contradictions: boolean;
};

type ObservedShape = {
  sections: string[];
  referenced_evidence_ids: string[];
  dropped_evidence_ids: string[];
  contradiction_strategy: string;
  clarification_prompt_present?: boolean;
  recovery_path_present?: boolean;
  unresolved_contradictions_remaining?: number;
};

type SynthesisCase = {
  id: string;
  evidence: EvidenceCase[];
  contradictions: ContradictionCase[];
  expected_answer_shape: ExpectedShape;
  observed_answer_shape: ObservedShape;
};

type SynthesisFixture = {
  schema_id: string;
  schema_version: number;
  cases: SynthesisCase[];
};

type CaseEvaluation = {
  id: string;
  ok: boolean;
  evidence_total: number;
  evidence_referenced: number;
  evidence_coverage_ratio: number;
  dropped_evidence_count: number;
  contradictions_total: number;
  contradictions_unresolved: number;
  contradiction_handled: boolean;
  unresolved_without_clarification_or_recovery: boolean;
  sections_missing: string[];
  required_evidence_missing: string[];
  failures: string[];
};

const DEFAULT_FIXTURE_PATH = 'tests/tooling/fixtures/synthesis_mixed_evidence_regression_matrix.json';
const DEFAULT_OUT_PATH = 'core/local/artifacts/synthesis_mixed_evidence_quality_current.json';
const DEFAULT_OUT_LATEST_PATH = 'artifacts/synthesis_mixed_evidence_quality_latest.json';
const DEFAULT_STATE_PATH = 'local/state/ops/synthesis_mixed_evidence/latest.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/SYNTHESIS_MIXED_EVIDENCE_QUALITY_CURRENT.md';

function readJson<T>(pathname: string): T | null {
  try {
    return JSON.parse(fs.readFileSync(pathname, 'utf8')) as T;
  } catch {
    return null;
  }
}

function writeJson(pathname: string, payload: unknown): void {
  fs.mkdirSync(path.dirname(pathname), { recursive: true });
  fs.writeFileSync(pathname, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function writeMarkdown(pathname: string, body: string): void {
  fs.mkdirSync(path.dirname(pathname), { recursive: true });
  fs.writeFileSync(pathname, body, 'utf8');
}

function canonicalStringList(raw: unknown, maxLen = 160): string[] {
  const rows = Array.isArray(raw) ? raw : [];
  return rows.map((value) => cleanText(value || '', maxLen)).filter(Boolean);
}

function unique<T>(rows: T[]): T[] {
  return Array.from(new Set(rows));
}

function evaluateCase(row: SynthesisCase): CaseEvaluation {
  const evidenceIds = unique(row.evidence.map((entry) => cleanText(entry.id || '', 120)).filter(Boolean));
  const sections = unique(canonicalStringList(row.observed_answer_shape?.sections, 120));
  const referenced = unique(canonicalStringList(row.observed_answer_shape?.referenced_evidence_ids, 120));
  const explicitDropped = unique(canonicalStringList(row.observed_answer_shape?.dropped_evidence_ids, 120));
  const requiredSections = unique(canonicalStringList(row.expected_answer_shape?.required_sections, 120));
  const requiredRefs = unique(canonicalStringList(row.expected_answer_shape?.must_reference_evidence_ids, 120));
  const maxDropped = Math.max(0, Number(row.expected_answer_shape?.max_dropped_evidence || 0));
  const unresolvedFromContradictions = row.contradictions.filter((entry) => entry.status === 'unresolved').length;
  const unresolvedRemaining = Math.max(
    unresolvedFromContradictions,
    Number(row.observed_answer_shape?.unresolved_contradictions_remaining || 0),
  );

  const sectionsMissing = requiredSections.filter((value) => !sections.includes(value));
  const requiredEvidenceMissing = requiredRefs.filter((value) => !referenced.includes(value));
  const impliedDropped = evidenceIds.filter((id) => !referenced.includes(id));
  const dropped = unique([...explicitDropped, ...impliedDropped]);
  const referencedKnown = referenced.filter((id) => evidenceIds.includes(id));
  const evidenceTotal = evidenceIds.length;
  const evidenceReferenced = referencedKnown.length;
  const evidenceCoverageRatio =
    evidenceTotal > 0 ? Number((evidenceReferenced / evidenceTotal).toFixed(6)) : 1.0;

  const contradictionStrategy = cleanText(row.observed_answer_shape?.contradiction_strategy || '', 80);
  const clarificationPresent = row.observed_answer_shape?.clarification_prompt_present === true;
  const recoveryPresent = row.observed_answer_shape?.recovery_path_present === true;
  const contradictionHandled =
    unresolvedRemaining === 0
    || clarificationPresent
    || recoveryPresent
    || contradictionStrategy === 'resolved'
    || contradictionStrategy === 'synthesized_resolution';
  const requiresClarification =
    row.expected_answer_shape?.requires_clarification_on_unresolved_contradictions === true
    && unresolvedRemaining > 0;
  const unresolvedWithoutClarificationOrRecovery =
    requiresClarification && !clarificationPresent && !recoveryPresent;

  const failures: string[] = [];
  if (sectionsMissing.length > 0) {
    failures.push(`sections_missing:${sectionsMissing.join(',')}`);
  }
  if (requiredEvidenceMissing.length > 0) {
    failures.push(`required_evidence_missing:${requiredEvidenceMissing.join(',')}`);
  }
  if (dropped.length > maxDropped) {
    failures.push(`dropped_evidence_exceeded:actual=${dropped.length}:max=${maxDropped}`);
  }
  if (!contradictionHandled) {
    failures.push('contradiction_unhandled');
  }
  if (unresolvedWithoutClarificationOrRecovery) {
    failures.push('unresolved_contradiction_missing_clarification_or_recovery');
  }

  return {
    id: cleanText(row.id || '', 120),
    ok: failures.length === 0,
    evidence_total: evidenceTotal,
    evidence_referenced: evidenceReferenced,
    evidence_coverage_ratio: evidenceCoverageRatio,
    dropped_evidence_count: dropped.length,
    contradictions_total: row.contradictions.length,
    contradictions_unresolved: unresolvedRemaining,
    contradiction_handled: contradictionHandled,
    unresolved_without_clarification_or_recovery: unresolvedWithoutClarificationOrRecovery,
    sections_missing: sectionsMissing,
    required_evidence_missing: requiredEvidenceMissing,
    failures,
  };
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Synthesis Mixed Evidence Quality (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report?.generated_at || '', 80)}`);
  lines.push(`- strict_mode: ${report?.strict_mode === true ? 'true' : 'false'}`);
  lines.push(`- ok: ${report?.ok === true ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Telemetry');
  lines.push(`- evidence_coverage_ratio: ${Number(report?.telemetry?.evidence_coverage_ratio || 0)}`);
  lines.push(
    `- contradiction_handling_rate: ${Number(report?.telemetry?.contradiction_handling_rate || 0)}`,
  );
  lines.push(`- dropped_evidence_count: ${Number(report?.telemetry?.dropped_evidence_count || 0)}`);
  lines.push(
    `- unresolved_without_clarification_or_recovery_count: ${Number(report?.telemetry?.unresolved_without_clarification_or_recovery_count || 0)}`,
  );
  lines.push('');
  lines.push('## Gate Checks');
  for (const row of Array.isArray(report?.gate_checks) ? report.gate_checks : []) {
    lines.push(
      `- ${cleanText(row?.id || 'unknown', 120)}: ${row?.ok === true ? 'pass' : 'fail'} (${cleanText(row?.detail || '', 240)})`,
    );
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT_PATH });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT_PATH, 400),
    outLatestPath: cleanText(readFlag(argv, 'out-latest') || DEFAULT_OUT_LATEST_PATH, 400),
    statePath: cleanText(readFlag(argv, 'state') || DEFAULT_STATE_PATH, 400),
    markdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MARKDOWN_PATH, 400),
    fixturePath: cleanText(readFlag(argv, 'fixture') || DEFAULT_FIXTURE_PATH, 400),
  };
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const fixtureAbs = path.resolve(root, args.fixturePath);
  const fixture = readJson<SynthesisFixture>(fixtureAbs);
  const failures: Array<{ id: string; detail: string }> = [];

  if (!fixture) {
    failures.push({ id: 'fixture_missing', detail: args.fixturePath });
  } else {
    if (cleanText(fixture.schema_id || '', 120) !== 'synthesis_mixed_evidence_regression_matrix') {
      failures.push({
        id: 'fixture_schema_id_invalid',
        detail: cleanText(fixture.schema_id || 'missing', 120),
      });
    }
    if (Number(fixture.schema_version || 0) !== 1) {
      failures.push({
        id: 'fixture_schema_version_invalid',
        detail: cleanText(String(fixture.schema_version ?? 'missing'), 40),
      });
    }
  }

  const rows = Array.isArray(fixture?.cases) ? fixture.cases : [];
  const evaluations = rows.map((row) => evaluateCase(row));
  for (const row of evaluations.filter((entry) => !entry.ok)) {
    failures.push({
      id: `case:${row.id}`,
      detail: cleanText(row.failures.join(';') || 'failed', 500),
    });
  }

  const evidenceTotal = evaluations.reduce((sum, row) => sum + row.evidence_total, 0);
  const evidenceReferenced = evaluations.reduce((sum, row) => sum + row.evidence_referenced, 0);
  const evidenceCoverageRatio =
    evidenceTotal > 0 ? Number((evidenceReferenced / evidenceTotal).toFixed(6)) : 1;
  const contradictionsTotal = evaluations.reduce((sum, row) => sum + row.contradictions_total, 0);
  const contradictionsHandled = evaluations.filter((row) => row.contradiction_handled).length;
  const contradictionCases = evaluations.filter((row) => row.contradictions_total > 0).length;
  const contradictionHandlingRate =
    contradictionCases > 0 ? Number((contradictionsHandled / contradictionCases).toFixed(6)) : 1;
  const droppedEvidenceCount = evaluations.reduce((sum, row) => sum + row.dropped_evidence_count, 0);
  const unresolvedWithoutGuardCount = evaluations.filter(
    (row) => row.unresolved_without_clarification_or_recovery,
  ).length;

  const gateChecks = [
    {
      id: 'synthesis_required_sections_contract',
      ok: evaluations.every((row) => row.sections_missing.length === 0),
      detail: `missing_cases=${evaluations.filter((row) => row.sections_missing.length > 0).length}`,
    },
    {
      id: 'synthesis_required_evidence_reference_contract',
      ok: evaluations.every((row) => row.required_evidence_missing.length === 0),
      detail: `missing_cases=${evaluations.filter((row) => row.required_evidence_missing.length > 0).length}`,
    },
    {
      id: 'synthesis_unresolved_contradiction_requires_clarification_or_recovery',
      ok: unresolvedWithoutGuardCount === 0,
      detail: `value=${unresolvedWithoutGuardCount};max=0`,
    },
    {
      id: 'synthesis_telemetry_evidence_coverage_ratio_nonzero',
      ok: evidenceCoverageRatio > 0,
      detail: `value=${evidenceCoverageRatio}`,
    },
    {
      id: 'synthesis_telemetry_contradiction_handling_rate_nonzero',
      ok: contradictionHandlingRate > 0,
      detail: `value=${contradictionHandlingRate}`,
    },
  ];

  const allChecksPass = failures.length === 0 && gateChecks.every((row) => row.ok);
  const report = {
    type: 'synthesis_mixed_evidence_quality',
    schema_version: 1,
    generated_at: new Date().toISOString(),
    strict_mode: args.strict,
    ok: allChecksPass,
    fixture_path: args.fixturePath,
    telemetry: {
      evidence_coverage_ratio: evidenceCoverageRatio,
      contradiction_handling_rate: contradictionHandlingRate,
      dropped_evidence_count: droppedEvidenceCount,
      unresolved_without_clarification_or_recovery_count: unresolvedWithoutGuardCount,
      contradictions_total: contradictionsTotal,
      case_count: evaluations.length,
    },
    summary: {
      total_cases: evaluations.length,
      passed_cases: evaluations.filter((row) => row.ok).length,
      failed_cases: evaluations.filter((row) => !row.ok).length,
    },
    gate_checks: gateChecks,
    case_results: evaluations,
    failures,
  };

  const outAbs = path.resolve(root, args.outPath);
  const latestAbs = path.resolve(root, args.outLatestPath);
  const stateAbs = path.resolve(root, args.statePath);
  const markdownAbs = path.resolve(root, args.markdownPath);
  writeJson(outAbs, report);
  writeJson(latestAbs, report);
  writeJson(stateAbs, report);
  writeMarkdown(markdownAbs, renderMarkdown(report));

  const exitCode = args.strict ? (allChecksPass ? 0 : 1) : 0;
  emitStructuredResult(
    {
      ok: allChecksPass,
      report_path: args.outPath,
      latest_path: args.outLatestPath,
      markdown_path: args.markdownPath,
      failures: failures.length,
    },
    { outPath: args.outPath },
  );
  return exitCode;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  process.exit(run());
}
