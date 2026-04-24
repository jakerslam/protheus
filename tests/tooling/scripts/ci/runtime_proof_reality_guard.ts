#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

type ProfileId = 'rich' | 'pure' | 'tiny-max';

type Failure = {
  id: string;
  detail: string;
};

const REQUIRED_PROFILES: ProfileId[] = ['rich', 'pure', 'tiny-max'];
const RUNTIME_EMPIRICAL_METRICS = [
  'peak_rss_mb',
  'queue_depth_max',
  'receipt_throughput_per_min',
  'receipt_p95_latency_ms',
  'conduit_recovery_ms',
];
const BOUNDEDNESS_TREND_METRICS = [
  'max_rss_mb',
  'queue_depth_max',
  'queue_depth_p95',
  'recovery_time_ms_max',
];

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/runtime_proof_reality_guard_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    markdownPath: cleanText(
      readFlag(argv, 'out-markdown') || 'local/workspace/reports/RUNTIME_PROOF_REALITY_GUARD_CURRENT.md',
      400,
    ),
    policyPath: cleanText(
      readFlag(argv, 'policy') || 'tests/tooling/config/runtime_empirical_coverage_policy.json',
      400,
    ),
    syntheticPath: cleanText(
      readFlag(argv, 'synthetic') || 'core/local/artifacts/runtime_proof_synthetic_canary_current.json',
      400,
    ),
    empiricalPath: cleanText(
      readFlag(argv, 'empirical') || 'core/local/artifacts/runtime_proof_empirical_release_evidence_current.json',
      400,
    ),
    trendsPath: cleanText(
      readFlag(argv, 'trends') || 'core/local/artifacts/runtime_proof_empirical_trends_current.json',
      400,
    ),
    coveragePath: cleanText(
      readFlag(argv, 'coverage') || 'core/local/artifacts/runtime_proof_empirical_profile_coverage_current.json',
      400,
    ),
    minimumContractPath: cleanText(
      readFlag(argv, 'minimum-contract') || 'core/local/artifacts/runtime_proof_empirical_minimum_contract_current.json',
      400,
    ),
    boundednessGatePath: cleanText(
      readFlag(argv, 'boundedness-gate') || 'core/local/artifacts/runtime_boundedness_release_gate_current.json',
      400,
    ),
  };
}

function readJson(filePath: string): any {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function safeNumber(value: unknown, fallback = 0): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function findProfile(rows: unknown, profile: ProfileId): any {
  if (!Array.isArray(rows)) return null;
  return rows.find((row: any) => cleanText(row?.profile || '', 40) === profile) || null;
}

function profileSet(rows: unknown): string[] {
  if (!Array.isArray(rows)) return [];
  return rows.map((row: any) => cleanText(row?.profile || '', 40)).filter(Boolean).sort();
}

function profileSetMatches(rows: unknown): boolean {
  return profileSet(rows).join(',') === [...REQUIRED_PROFILES].sort().join(',');
}

function finiteDeltaOk(row: any, currentKey = 'current', previousKey = 'previous', deltaKey = 'delta'): boolean {
  const current = Number(row?.[currentKey]);
  const previous = Number(row?.[previousKey]);
  const delta = Number(row?.[deltaKey]);
  if (!Number.isFinite(current) || !Number.isFinite(previous) || !Number.isFinite(delta)) return false;
  return Math.abs(current - previous - delta) <= 0.000001;
}

function validExemption(raw: any, profile: ProfileId, nowMs: number): boolean {
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return false;
  if (cleanText(raw.profile || '', 40) !== profile) return false;
  const expiresAt = cleanText(raw.expires_at || '', 80);
  const expiresMs = Date.parse(expiresAt);
  return (
    Number.isFinite(expiresMs) &&
    expiresMs > nowMs &&
    cleanText(raw.reason || '', 200).length > 0 &&
    cleanText(raw.owner || '', 120).length > 0 &&
    cleanText(raw.evidence || '', 240).length > 0
  );
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Runtime Proof Reality Guard (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report?.generated_at || '', 80)}`);
  lines.push(`- revision: ${cleanText(report?.revision || '', 120)}`);
  lines.push(`- pass: ${report?.ok === true ? 'true' : 'false'}`);
  lines.push(`- failures: ${Number(report?.summary?.failure_count || 0)}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- synthetic_empirical_separated: ${report?.summary?.synthetic_empirical_separated === true ? 'true' : 'false'}`);
  lines.push(`- empirical_profiles_covered: ${Number(report?.summary?.empirical_profiles_covered || 0)}/${Number(report?.summary?.empirical_profiles_total || 0)}`);
  lines.push(`- runtime_trend_delta_count: ${Number(report?.summary?.runtime_trend_delta_count || 0)}`);
  lines.push(`- boundedness_trend_delta_count: ${Number(report?.summary?.boundedness_trend_delta_count || 0)}`);
  const failures = Array.isArray(report?.failures) ? report.failures : [];
  if (failures.length > 0) {
    lines.push('');
    lines.push('## Failures');
    for (const failure of failures) {
      lines.push(`- ${cleanText(failure?.id || 'unknown', 120)}: ${cleanText(failure?.detail || '', 260)}`);
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function writeText(filePath: string, body: string): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, body, 'utf8');
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const failures: Failure[] = [];
  const nowMs = Date.now();

  let policy: any = null;
  let synthetic: any = null;
  let empirical: any = null;
  let trends: any = null;
  let coverage: any = null;
  let minimumContract: any = null;
  let boundednessGate: any = null;

  for (const [name, relPath, assign] of [
    ['policy', args.policyPath, (payload: any) => (policy = payload)],
    ['synthetic', args.syntheticPath, (payload: any) => (synthetic = payload)],
    ['empirical', args.empiricalPath, (payload: any) => (empirical = payload)],
    ['trends', args.trendsPath, (payload: any) => (trends = payload)],
    ['coverage', args.coveragePath, (payload: any) => (coverage = payload)],
    ['minimum_contract', args.minimumContractPath, (payload: any) => (minimumContract = payload)],
    ['boundedness_gate', args.boundednessGatePath, (payload: any) => (boundednessGate = payload)],
  ] as Array<[string, string, (payload: any) => void]>) {
    try {
      assign(readJson(path.resolve(root, relPath)));
    } catch (error) {
      failures.push({
        id: 'runtime_proof_reality_input_missing_or_invalid',
        detail: `${name}:${relPath}:${cleanText((error as Error)?.message || 'read_failed', 180)}`,
      });
    }
  }

  if (cleanText(policy?.schema_id || '', 120) !== 'runtime_empirical_coverage_policy.v1') {
    failures.push({ id: 'empirical_coverage_policy_schema_invalid', detail: args.policyPath });
  }
  if ((policy?.supported_profiles || []).join(',') !== REQUIRED_PROFILES.join(',')) {
    failures.push({ id: 'empirical_coverage_policy_profile_set_invalid', detail: String(policy?.supported_profiles || []) });
  }

  const syntheticSeparated =
    synthetic?.type === 'runtime_proof_synthetic_canary' &&
    empirical?.type === 'runtime_proof_empirical_release_evidence' &&
    synthetic?.type !== empirical?.type;
  if (!syntheticSeparated) {
    failures.push({ id: 'synthetic_empirical_artifacts_not_separated', detail: `${args.syntheticPath};${args.empiricalPath}` });
  }
  for (const profile of REQUIRED_PROFILES) {
    const syntheticRow = findProfile(synthetic?.profiles, profile);
    const empiricalRow = findProfile(empirical?.profiles, profile);
    if (!syntheticRow) failures.push({ id: 'synthetic_profile_missing', detail: profile });
    if (!empiricalRow) failures.push({ id: 'empirical_profile_missing', detail: profile });
    if (syntheticRow && (Array.isArray(syntheticRow.sources) || Array.isArray(syntheticRow.provided_keys))) {
      failures.push({ id: 'synthetic_artifact_contains_empirical_fields', detail: profile });
    }
    if (empiricalRow && (empiricalRow.synthetic_metrics || empiricalRow.effective_metrics || empiricalRow.proof_tracks)) {
      failures.push({ id: 'empirical_artifact_contains_merged_fields', detail: profile });
    }
  }

  if (!profileSetMatches(coverage?.profiles)) {
    failures.push({ id: 'empirical_coverage_profile_set_invalid', detail: profileSet(coverage?.profiles).join(',') });
  }
  if (!profileSetMatches(minimumContract?.profiles)) {
    failures.push({ id: 'empirical_minimum_contract_profile_set_invalid', detail: profileSet(minimumContract?.profiles).join(',') });
  }

  const profileCoverageRows = REQUIRED_PROFILES.map((profile) => {
    const coverageRow = findProfile(coverage?.profiles, profile) || {};
    const minimumRow = findProfile(minimumContract?.profiles, profile) || {};
    const empiricalRow = findProfile(empirical?.profiles, profile) || {};
    const policyMin = safeNumber(policy?.min_empirical_sample_points?.[profile], 0);
    const minRequired = Math.max(policyMin, safeNumber(minimumRow?.empirical_min_sample_points_required, 0));
    const samplePoints = Math.max(
      safeNumber(coverageRow?.empirical_sample_points, 0),
      safeNumber(empiricalRow?.sample_points, 0),
    );
    const exemption = (Array.isArray(policy?.exemptions) ? policy.exemptions : []).find((row: any) =>
      validExemption(row, profile, nowMs),
    );
    const covered = samplePoints >= minRequired;
    if (!covered && !exemption) {
      failures.push({ id: 'empirical_profile_coverage_missing', detail: `${profile}:samples=${samplePoints};min=${minRequired}` });
    }
    if (minimumRow?.empirical_profile_minimum_configured !== true && !exemption) {
      failures.push({ id: 'empirical_profile_minimum_not_configured', detail: profile });
    }
    for (const [field, failureId] of [
      ['empirical_required_sources_ok', 'empirical_required_sources_missing'],
      ['empirical_required_metrics_ok', 'empirical_required_metrics_missing'],
      ['empirical_required_positive_metrics_ok', 'empirical_required_positive_metrics_missing'],
    ] as Array<[string, string]>) {
      if (coverageRow?.[field] !== true && !exemption) {
        failures.push({ id: failureId, detail: profile });
      }
    }
    return { profile, sample_points: samplePoints, min_required: minRequired, covered, exemption_active: !!exemption };
  });

  if (trends?.type !== 'runtime_proof_empirical_trends') {
    failures.push({ id: 'runtime_empirical_trends_type_invalid', detail: args.trendsPath });
  }
  if (trends?.baseline_available !== true || safeNumber(trends?.history_samples, 0) < 2) {
    failures.push({ id: 'runtime_empirical_trend_baseline_missing', detail: `history_samples=${safeNumber(trends?.history_samples, 0)}` });
  }
  const runtimeTrendDeltas: any[] = [];
  for (const profile of REQUIRED_PROFILES) {
    const trendRow = findProfile(trends?.profiles, profile);
    if (!trendRow) {
      failures.push({ id: 'runtime_empirical_trend_profile_missing', detail: profile });
      continue;
    }
    for (const metric of RUNTIME_EMPIRICAL_METRICS) {
      const delta = trendRow?.metric_deltas?.[metric];
      const windowDelta = trendRow?.metric_window_deltas?.[metric];
      if (!finiteDeltaOk(delta)) {
        failures.push({ id: 'runtime_empirical_trend_delta_invalid', detail: `${profile}:${metric}` });
      } else {
        runtimeTrendDeltas.push({ profile, metric, ...delta });
      }
      if (!finiteDeltaOk(windowDelta, 'current', 'baseline', 'delta_from_baseline')) {
        failures.push({ id: 'runtime_empirical_trend_window_delta_invalid', detail: `${profile}:${metric}` });
      }
    }
  }

  if (boundednessGate?.type !== 'runtime_boundedness_release_gate') {
    failures.push({ id: 'boundedness_release_gate_type_invalid', detail: args.boundednessGatePath });
  }
  if (boundednessGate?.ok !== true) {
    failures.push({ id: 'boundedness_release_gate_not_passing', detail: args.boundednessGatePath });
  }
  const boundednessDiffRows = Array.isArray(boundednessGate?.boundedness_regression_diff)
    ? boundednessGate.boundedness_regression_diff
    : [];
  const boundednessTrendDeltas: any[] = [];
  for (const profile of REQUIRED_PROFILES) {
    for (const metric of BOUNDEDNESS_TREND_METRICS) {
      const row = boundednessDiffRows.find(
        (candidate: any) => cleanText(candidate?.profile || '', 40) === profile && cleanText(candidate?.metric || '', 80) === metric,
      );
      if (!row || !finiteDeltaOk(row, 'current', 'baseline', 'delta')) {
        failures.push({ id: 'boundedness_trend_delta_missing_or_invalid', detail: `${profile}:${metric}` });
      } else {
        boundednessTrendDeltas.push({ profile, metric, current: row.current, baseline: row.baseline, delta: row.delta, ok: row.ok === true });
      }
    }
  }

  const report = {
    ok: failures.length === 0,
    type: 'runtime_proof_reality_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    inputs: {
      policy_path: args.policyPath,
      synthetic_path: args.syntheticPath,
      empirical_path: args.empiricalPath,
      trends_path: args.trendsPath,
      coverage_path: args.coveragePath,
      minimum_contract_path: args.minimumContractPath,
      boundedness_gate_path: args.boundednessGatePath,
    },
    summary: {
      synthetic_empirical_separated: syntheticSeparated,
      empirical_profiles_total: REQUIRED_PROFILES.length,
      empirical_profiles_covered: profileCoverageRows.filter((row) => row.covered || row.exemption_active).length,
      runtime_trend_delta_count: runtimeTrendDeltas.length,
      boundedness_trend_delta_count: boundednessTrendDeltas.length,
      failure_count: failures.length,
    },
    empirical_profile_coverage: profileCoverageRows,
    runtime_empirical_trend_deltas: runtimeTrendDeltas,
    boundedness_trend_deltas: boundednessTrendDeltas,
    failures,
    artifact_paths: [args.markdownPath],
  };

  writeText(path.resolve(root, args.markdownPath), renderMarkdown(report));
  return emitStructuredResult(report, {
    outPath: args.outPath,
    strict: args.strict,
    ok: report.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run };
