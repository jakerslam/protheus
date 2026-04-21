#!/usr/bin/env tsx

import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeJsonArtifact } from '../../lib/result.ts';
import { run as runHarness } from './runtime_proof_harness.ts';
import { run as runReleaseGate } from './runtime_proof_release_gate.ts';
import { run as runAdapterChaosGate } from './adapter_runtime_chaos_gate.ts';
import { run as runBoundednessInspect } from './runtime_boundedness_inspect.ts';

type ProfileId = 'rich' | 'pure' | 'tiny-max';
type ProfileSelector = ProfileId | 'all';
type ProofTrackId = 'synthetic' | 'empirical' | 'dual';
type EmpiricalTrendMetricKey =
  | 'peak_rss_mb'
  | 'queue_depth_max'
  | 'receipt_throughput_per_min'
  | 'receipt_p95_latency_ms'
  | 'conduit_recovery_ms';

const EMPIRICAL_TREND_METRIC_KEYS: EmpiricalTrendMetricKey[] = [
  'peak_rss_mb',
  'queue_depth_max',
  'receipt_throughput_per_min',
  'receipt_p95_latency_ms',
  'conduit_recovery_ms',
];

function parseProfile(raw: string | undefined): ProfileSelector | null {
  const normalized = cleanText(raw || 'all', 32).toLowerCase();
  if (normalized === 'all') return 'all';
  if (normalized === 'rich') return 'rich';
  if (normalized === 'pure') return 'pure';
  if (normalized === 'tiny-max' || normalized === 'tiny' || normalized === 'tiny_max') {
    return 'tiny-max';
  }
  return null;
}

function parseProofTrack(raw: string | undefined): ProofTrackId {
  const normalized = cleanText(raw || 'dual', 24).toLowerCase();
  if (normalized === 'synthetic') return 'synthetic';
  if (normalized === 'empirical') return 'empirical';
  return 'dual';
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/runtime_proof_verify_current.json',
  });
  const profile = parseProfile(readFlag(argv, 'profile'));
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    profile,
    proofTrack: parseProofTrack(readFlag(argv, 'proof-track')),
    empiricalHistoryPath: cleanText(
      readFlag(argv, 'empirical-history') || 'core/local/state/ops/runtime_proof_empirical_history.jsonl',
      400,
    ),
  };
}

function readJsonBestEffort(filePath: string): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function sha256File(filePath: string): string {
  try {
    return createHash('sha256').update(fs.readFileSync(filePath)).digest('hex');
  } catch {
    return '';
  }
}

function safeNumber(value: unknown, fallback = 0): number {
  const num = Number(value);
  return Number.isFinite(num) ? num : fallback;
}

function toStringArray(value: unknown, limit = 120): string[] {
  if (!Array.isArray(value)) return [];
  const out: string[] = [];
  for (const item of value) {
    const cleaned = cleanText(String(item ?? ''), limit);
    if (cleaned.length > 0) out.push(cleaned);
  }
  return out;
}

function uniqueStringValues(values: string[]): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const value of values) {
    const cleaned = cleanText(value, 120);
    if (!cleaned || seen.has(cleaned)) continue;
    seen.add(cleaned);
    out.push(cleaned);
  }
  return out;
}

function ensureParentDir(filePath: string): void {
  const parent = path.dirname(filePath);
  fs.mkdirSync(parent, { recursive: true });
}

function readJsonLinesBestEffort(filePath: string): any[] {
  try {
    const raw = fs.readFileSync(filePath, 'utf8');
    return raw
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => {
        try {
          return JSON.parse(line);
        } catch {
          return null;
        }
      })
      .filter((row) => !!row);
  } catch {
    return [];
  }
}

function appendJsonLine(filePath: string, payload: unknown): void {
  ensureParentDir(filePath);
  fs.appendFileSync(filePath, `${JSON.stringify(payload)}\n`, 'utf8');
}

function renderEmpiricalProfileCoverageMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Runtime Proof Empirical Profile Coverage (Current)');
  lines.push('');
  lines.push(`Generated: ${cleanText(payload?.generated_at || '', 80)}`);
  lines.push(`Revision: ${cleanText(payload?.revision || '', 120)}`);
  lines.push(`Proof track: ${cleanText(payload?.proof_track || '', 32)}`);
  lines.push(`Pass: ${payload?.ok === true ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Profiles');
  const profiles = Array.isArray(payload?.profiles) ? payload.profiles : [];
  for (const profile of profiles) {
    lines.push(`- ${cleanText(profile?.profile || 'unknown', 40)}:`);
    lines.push(
      `  - empirical_sample_points: ${safeNumber(profile?.empirical_sample_points, 0)}`,
    );
    lines.push(
      `  - empirical_min_sample_points_required: ${safeNumber(profile?.empirical_min_sample_points_required, 0)}`,
    );
    lines.push(
      `  - empirical_sample_points_ok: ${profile?.empirical_sample_points_ok === true ? 'true' : 'false'}`,
    );
    lines.push(
      `  - empirical_provided_keys_count: ${safeNumber(profile?.empirical_provided_keys_count, 0)}`,
    );
    lines.push(
      `  - empirical_sources_count: ${safeNumber(profile?.empirical_sources_count, 0)}`,
    );
    lines.push(
      `  - empirical_required_sources_ok: ${profile?.empirical_required_sources_ok === true ? 'true' : 'false'}`,
    );
    lines.push(
      `  - empirical_required_metrics_ok: ${profile?.empirical_required_metrics_ok === true ? 'true' : 'false'}`,
    );
    lines.push(
      `  - empirical_required_positive_metrics_ok: ${
        profile?.empirical_required_positive_metrics_ok === true ? 'true' : 'false'
      }`,
    );
    const missingSources = toStringArray(profile?.empirical_required_sources_missing, 120);
    const missingMetrics = toStringArray(profile?.empirical_required_metrics_missing, 120);
    const nonPositiveMetrics = toStringArray(
      profile?.empirical_required_positive_metrics_missing,
      120,
    );
    if (missingSources.length > 0) {
      lines.push(`  - empirical_required_sources_missing: ${missingSources.join(', ')}`);
    }
    if (missingMetrics.length > 0) {
      lines.push(`  - empirical_required_metrics_missing: ${missingMetrics.join(', ')}`);
    }
    if (nonPositiveMetrics.length > 0) {
      lines.push(
        `  - empirical_required_positive_metrics_missing: ${nonPositiveMetrics.join(', ')}`,
      );
    }
    const sources = Array.isArray(profile?.empirical_source_rows) ? profile.empirical_source_rows : [];
    const requiredSourceRows = Array.isArray(profile?.empirical_required_source_rows)
      ? profile.empirical_required_source_rows
      : [];
    const requiredMetricRows = Array.isArray(profile?.empirical_required_metric_rows)
      ? profile.empirical_required_metric_rows
      : [];
    if (sources.length > 0) {
      lines.push('  - empirical_source_rows:');
      for (const source of sources) {
        lines.push(
          `    - ${cleanText(source?.id || 'unknown', 80)} (loaded=${
            source?.loaded === true ? 'true' : 'false'
          }, sample_points=${safeNumber(source?.sample_points, 0)}, artifact=${cleanText(
            source?.artifact_path || '',
            200,
          ) || 'n/a'})`,
        );
      }
    }
    if (requiredSourceRows.length > 0) {
      lines.push('  - empirical_required_source_rows:');
      for (const row of requiredSourceRows) {
        lines.push(
          `    - ${cleanText(row?.id || 'unknown', 80)} (missing=${
            row?.required_missing === true ? 'true' : 'false'
          }, satisfied=${row?.required_satisfied === true ? 'true' : 'false'}, loaded=${
            row?.loaded === true ? 'true' : 'false'
          }, sample_points=${safeNumber(row?.sample_points, 0)})`,
        );
      }
    }
    if (requiredMetricRows.length > 0) {
      lines.push('  - empirical_required_metric_rows:');
      for (const row of requiredMetricRows) {
        lines.push(
          `    - ${cleanText(row?.key || 'unknown', 80)} (missing=${
            row?.required_missing === true ? 'true' : 'false'
          }, non_positive=${row?.required_non_positive === true ? 'true' : 'false'}, satisfied=${
            row?.required_satisfied === true ? 'true' : 'false'
          }, value=${safeNumber(row?.value, 0)})`,
        );
      }
    }
    lines.push(`  - source_artifact: ${cleanText(profile?.source_artifact || '', 240)}`);
    lines.push(
      `  - release_gate_artifact: ${cleanText(profile?.release_gate_artifact || '', 240)}`,
    );
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function renderEmpiricalSourceMatrixMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Runtime Proof Empirical Source Matrix (Current)');
  lines.push('');
  lines.push(`Generated: ${cleanText(payload?.generated_at || '', 80)}`);
  lines.push(`Revision: ${cleanText(payload?.revision || '', 120)}`);
  lines.push(`Proof track: ${cleanText(payload?.proof_track || '', 32)}`);
  lines.push(`Pass: ${payload?.ok === true ? 'true' : 'false'}`);
  lines.push('');
  const profiles = Array.isArray(payload?.profiles) ? payload.profiles : [];
  for (const profile of profiles) {
    lines.push(`## ${cleanText(profile?.profile || 'unknown', 40)}`);
    lines.push(
      `- empirical_sample_points: ${safeNumber(profile?.empirical_sample_points, 0)} (min_required=${safeNumber(
        profile?.empirical_min_sample_points_required,
        0,
      )}, ok=${profile?.empirical_sample_points_ok === true ? 'true' : 'false'})`,
    );
    lines.push(
      `- required_sources_ok: ${profile?.empirical_required_sources_ok === true ? 'true' : 'false'}`,
    );
    lines.push(
      `- required_metrics_ok: ${profile?.empirical_required_metrics_ok === true ? 'true' : 'false'}`,
    );
    lines.push(
      `- required_positive_metrics_ok: ${
        profile?.empirical_required_positive_metrics_ok === true ? 'true' : 'false'
      }`,
    );
    const missingSources = toStringArray(profile?.empirical_required_sources_missing, 120);
    const missingMetrics = toStringArray(profile?.empirical_required_metrics_missing, 120);
    const nonPositiveMetrics = toStringArray(
      profile?.empirical_required_positive_metrics_missing,
      120,
    );
    lines.push(
      `- missing_required_sources: ${missingSources.length > 0 ? missingSources.join(', ') : 'none'}`,
    );
    lines.push(
      `- missing_required_metrics: ${missingMetrics.length > 0 ? missingMetrics.join(', ') : 'none'}`,
    );
    lines.push(
      `- non_positive_required_metrics: ${
        nonPositiveMetrics.length > 0 ? nonPositiveMetrics.join(', ') : 'none'
      }`,
    );
    const sources = Array.isArray(profile?.empirical_source_rows) ? profile.empirical_source_rows : [];
    const requiredSourceRows = Array.isArray(profile?.empirical_required_source_rows)
      ? profile.empirical_required_source_rows
      : [];
    const requiredMetricRows = Array.isArray(profile?.empirical_required_metric_rows)
      ? profile.empirical_required_metric_rows
      : [];
    if (sources.length > 0) {
      lines.push('- sources:');
      for (const source of sources) {
        lines.push(
          `  - ${cleanText(source?.id || 'unknown', 80)} (loaded=${
            source?.loaded === true ? 'true' : 'false'
          }, sample_points=${safeNumber(source?.sample_points, 0)}, artifact=${cleanText(
            source?.artifact_path || '',
            200,
          ) || 'n/a'})`,
        );
      }
    } else {
      lines.push('- sources: none');
    }
    if (requiredSourceRows.length > 0) {
      lines.push('- required_source_rows:');
      for (const sourceRow of requiredSourceRows) {
        lines.push(
          `  - ${cleanText(sourceRow?.id || 'unknown', 80)} (missing=${
            sourceRow?.required_missing === true ? 'true' : 'false'
          }, satisfied=${sourceRow?.required_satisfied === true ? 'true' : 'false'}, loaded=${
            sourceRow?.loaded === true ? 'true' : 'false'
          }, sample_points=${safeNumber(sourceRow?.sample_points, 0)})`,
        );
      }
    } else {
      lines.push('- required_source_rows: none');
    }
    if (requiredMetricRows.length > 0) {
      lines.push('- required_metric_rows:');
      for (const metricRow of requiredMetricRows) {
        lines.push(
          `  - ${cleanText(metricRow?.key || 'unknown', 80)} (missing=${
            metricRow?.required_missing === true ? 'true' : 'false'
          }, non_positive=${metricRow?.required_non_positive === true ? 'true' : 'false'}, satisfied=${
            metricRow?.required_satisfied === true ? 'true' : 'false'
          }, value=${safeNumber(metricRow?.value, 0)})`,
        );
      }
    } else {
      lines.push('- required_metric_rows: none');
    }
    lines.push('');
  }
  return `${lines.join('\n')}\n`;
}

function renderEmpiricalTrendsMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Runtime Proof Empirical Trends (Current)');
  lines.push('');
  lines.push(`Generated: ${cleanText(payload?.generated_at || '', 80)}`);
  lines.push(`Revision: ${cleanText(payload?.revision || '', 120)}`);
  lines.push(`Proof track: ${cleanText(payload?.proof_track || '', 32)}`);
  lines.push(`Pass: ${payload?.ok === true ? 'true' : 'false'}`);
  lines.push(`History samples: ${safeNumber(payload?.history_samples, 0)}`);
  lines.push(`Baseline available: ${payload?.baseline_available === true ? 'true' : 'false'}`);
  lines.push('');
  const profiles = Array.isArray(payload?.profiles) ? payload.profiles : [];
  for (const profile of profiles) {
    lines.push(`## ${cleanText(profile?.profile || 'unknown', 40)}`);
    lines.push(
      `- sample_points: ${safeNumber(profile?.sample_points, 0)} (delta=${safeNumber(
        profile?.sample_points_delta,
        0,
      )})`,
    );
    const metricDeltas = profile?.metric_deltas || {};
    const metricEntries = Object.entries(metricDeltas).filter(
      (entry) => cleanText(String(entry[0] || ''), 120).length > 0,
    );
    if (metricEntries.length > 0) {
      lines.push('- metric_deltas:');
      for (const [metricKey, deltaPayload] of metricEntries) {
        const currentValue = safeNumber((deltaPayload as any)?.current, 0);
        const previousValue = safeNumber((deltaPayload as any)?.previous, 0);
        const deltaValue = safeNumber((deltaPayload as any)?.delta, 0);
        lines.push(
          `  - ${cleanText(metricKey, 120)}: current=${currentValue}, previous=${previousValue}, delta=${deltaValue}`,
        );
      }
    } else {
      lines.push('- metric_deltas: none');
    }
    lines.push('');
  }
  return `${lines.join('\n')}\n`;
}

function renderEmpiricalReleaseEvidenceMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Runtime Proof Empirical Release Evidence (Current)');
  lines.push('');
  lines.push(`Generated: ${cleanText(payload?.generated_at || '', 80)}`);
  lines.push(`Revision: ${cleanText(payload?.revision || '', 120)}`);
  lines.push(`Proof track: ${cleanText(payload?.proof_track || '', 32)}`);
  lines.push(`Pass: ${payload?.ok === true ? 'true' : 'false'}`);
  lines.push('');
  const profiles = Array.isArray(payload?.profiles) ? payload.profiles : [];
  for (const profile of profiles) {
    lines.push(`## ${cleanText(profile?.profile || 'unknown', 40)}`);
    lines.push(`- sample_points: ${safeNumber(profile?.sample_points, 0)}`);
    lines.push(
      `- release_gate_ok: ${profile?.release_gate_ok === true ? 'true' : 'false'}`,
    );
    const providedKeys = toStringArray(profile?.provided_keys, 120);
    lines.push(`- provided_keys_count: ${providedKeys.length}`);
    const sources = Array.isArray(profile?.sources) ? profile.sources : [];
    lines.push(`- sources_count: ${sources.length}`);
    lines.push(`- source_artifact: ${cleanText(profile?.source_artifact || '', 240)}`);
    lines.push('');
  }
  return `${lines.join('\n')}\n`;
}

function renderEmpiricalProfileGateMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Runtime Proof Empirical Profile Gate (Current)');
  lines.push('');
  lines.push(`Generated: ${cleanText(payload?.generated_at || '', 80)}`);
  lines.push(`Revision: ${cleanText(payload?.revision || '', 120)}`);
  lines.push(`Proof track: ${cleanText(payload?.proof_track || '', 32)}`);
  lines.push(`Pass: ${payload?.ok === true ? 'true' : 'false'}`);
  const summary = payload?.summary || {};
  lines.push(`Profiles total: ${safeNumber(summary?.profiles_total, 0)}`);
  lines.push(`Profiles passed: ${safeNumber(summary?.profiles_passed, 0)}`);
  lines.push(`Profiles failed: ${safeNumber(summary?.profiles_failed, 0)}`);
  const failingProfiles = toStringArray(summary?.failing_profiles, 80);
  lines.push(`Failing profiles: ${failingProfiles.length > 0 ? failingProfiles.join(', ') : 'none'}`);
  const reasonBuckets = summary?.reason_buckets || {};
  const reasonEntries = Object.entries(reasonBuckets).filter(
    (entry) => cleanText(String(entry[0] || ''), 120).length > 0,
  );
  if (reasonEntries.length > 0) {
    lines.push('Reason buckets:');
    for (const [reason, count] of reasonEntries) {
      lines.push(`- ${cleanText(reason, 120)}: ${safeNumber(count, 0)}`);
    }
  }
  lines.push('');
  const profiles = Array.isArray(payload?.profiles) ? payload.profiles : [];
  for (const profile of profiles) {
    lines.push(`## ${cleanText(profile?.profile || 'unknown', 40)}`);
    lines.push(
      `- empirical_release_gate_pass: ${
        profile?.empirical_release_gate_pass === true ? 'true' : 'false'
      }`,
    );
    lines.push(
      `- empirical_sample_points: ${safeNumber(profile?.empirical_sample_points, 0)} (min_required=${safeNumber(
        profile?.empirical_min_sample_points_required,
        0,
      )}, ok=${profile?.empirical_sample_points_ok === true ? 'true' : 'false'})`,
    );
    lines.push(
      `- required_sources_ok: ${profile?.empirical_required_sources_ok === true ? 'true' : 'false'}`,
    );
    lines.push(
      `- required_metrics_ok: ${profile?.empirical_required_metrics_ok === true ? 'true' : 'false'}`,
    );
    lines.push(
      `- required_positive_metrics_ok: ${
        profile?.empirical_required_positive_metrics_ok === true ? 'true' : 'false'
      }`,
    );
    lines.push(
      `- release_gate_execution_ok: ${
        profile?.empirical_release_gate_execution_ok === true ? 'true' : 'false'
      }`,
    );
    const blockers = toStringArray(profile?.empirical_release_gate_reasons, 160);
    lines.push(`- blockers: ${blockers.length > 0 ? blockers.join('; ') : 'none'}`);
    const failedChecks = toStringArray(profile?.empirical_release_gate_checks_failed, 160);
    lines.push(`- failed_release_gate_checks: ${failedChecks.length > 0 ? failedChecks.join(', ') : 'none'}`);
    lines.push('');
  }
  return `${lines.join('\n')}\n`;
}

function renderEmpiricalProfileGateFailuresMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Runtime Proof Empirical Profile Gate Failures (Current)');
  lines.push('');
  lines.push(`Generated: ${cleanText(payload?.generated_at || '', 80)}`);
  lines.push(`Revision: ${cleanText(payload?.revision || '', 120)}`);
  lines.push(`Proof track: ${cleanText(payload?.proof_track || '', 32)}`);
  lines.push(`Has failures: ${payload?.has_failures === true ? 'true' : 'false'}`);
  lines.push(`Failing profiles count: ${safeNumber(payload?.failing_profiles_count, 0)}`);
  lines.push('');
  const failures = Array.isArray(payload?.failures) ? payload.failures : [];
  if (failures.length === 0) {
    lines.push('No empirical profile gate failures.');
    lines.push('');
    return `${lines.join('\n')}\n`;
  }
  for (const failure of failures) {
    lines.push(`## ${cleanText(failure?.profile || 'unknown', 40)}`);
    const reasons = toStringArray(failure?.reasons, 160);
    lines.push(`- reasons: ${reasons.length > 0 ? reasons.join('; ') : 'none'}`);
    const failedChecks = toStringArray(failure?.release_gate_checks_failed, 160);
    lines.push(`- release_gate_checks_failed: ${failedChecks.length > 0 ? failedChecks.join(', ') : 'none'}`);
    const missingSources = toStringArray(failure?.required_sources_missing, 120);
    lines.push(`- required_sources_missing: ${missingSources.length > 0 ? missingSources.join(', ') : 'none'}`);
    const missingMetrics = toStringArray(failure?.required_metrics_missing, 120);
    lines.push(`- required_metrics_missing: ${missingMetrics.length > 0 ? missingMetrics.join(', ') : 'none'}`);
    const nonPositiveMetrics = toStringArray(
      failure?.required_positive_metrics_missing,
      120,
    );
    lines.push(
      `- required_positive_metrics_missing: ${
        nonPositiveMetrics.length > 0 ? nonPositiveMetrics.join(', ') : 'none'
      }`,
    );
    lines.push(
      `- empirical_sample_points: ${safeNumber(failure?.empirical_sample_points, 0)} (min_required=${safeNumber(
        failure?.empirical_min_sample_points_required,
        0,
      )})`,
    );
    lines.push(
      `- artifacts: source=${cleanText(failure?.source_artifact || '', 220) || 'n/a'}; release_gate=${cleanText(
        failure?.release_gate_artifact || '',
        220,
      ) || 'n/a'}`,
    );
    lines.push('');
  }
  return `${lines.join('\n')}\n`;
}

function renderEmpiricalProfileReadinessMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Runtime Proof Empirical Profile Readiness (Current)');
  lines.push('');
  lines.push(`Generated: ${cleanText(payload?.generated_at || '', 80)}`);
  lines.push(`Revision: ${cleanText(payload?.revision || '', 120)}`);
  lines.push(`Proof track: ${cleanText(payload?.proof_track || '', 32)}`);
  lines.push(`Pass: ${payload?.ok === true ? 'true' : 'false'}`);
  const summary = payload?.summary || {};
  lines.push(`Profiles total: ${safeNumber(summary?.profiles_total, 0)}`);
  lines.push(`Release-ready profiles: ${safeNumber(summary?.release_ready_profiles, 0)}`);
  lines.push(`Degraded profiles: ${safeNumber(summary?.degraded_profiles, 0)}`);
  lines.push(`Blocked profiles: ${safeNumber(summary?.blocked_profiles, 0)}`);
  lines.push(`Average readiness score: ${safeNumber(summary?.readiness_score_avg, 0)}`);
  lines.push('');
  const profiles = Array.isArray(payload?.profiles) ? payload.profiles : [];
  for (const profile of profiles) {
    lines.push(`## ${cleanText(profile?.profile || 'unknown', 40)}`);
    lines.push(`- readiness_class: ${cleanText(profile?.readiness_class || '', 32) || 'unknown'}`);
    lines.push(`- readiness_score: ${safeNumber(profile?.readiness_score, 0)}`);
    const reasons = toStringArray(profile?.reasons, 160);
    lines.push(`- reasons: ${reasons.length > 0 ? reasons.join('; ') : 'none'}`);
    const severeReasons = toStringArray(profile?.severe_reasons, 160);
    lines.push(`- severe_reasons: ${severeReasons.length > 0 ? severeReasons.join('; ') : 'none'}`);
    const failedChecks = toStringArray(profile?.release_gate_checks_failed, 160);
    lines.push(`- release_gate_checks_failed: ${failedChecks.length > 0 ? failedChecks.join(', ') : 'none'}`);
    lines.push('');
  }
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  if (!args.profile) {
    const payload = {
      ok: false,
      type: 'runtime_proof_verify',
      error: 'runtime_proof_profile_invalid',
      profile: cleanText(readFlag(argv, 'profile') || '', 40),
      allowed_profiles: ['all', 'rich', 'pure', 'tiny-max'],
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  const profiles: ProfileId[] =
    args.profile === 'all' ? (['rich', 'pure', 'tiny-max'] as ProfileId[]) : [args.profile];

  const profileRuns = profiles.map((profile) => {
    const harnessOut = `core/local/artifacts/runtime_proof_harness_${profile}_current.json`;
    const harnessMetricsOut = `core/local/artifacts/runtime_proof_metrics_${profile}_current.json`;
    const gateOut = `core/local/artifacts/runtime_proof_release_gate_${profile}_current.json`;
    const gateMetricsOut = `core/local/artifacts/runtime_proof_release_metrics_${profile}_current.json`;
    const gateTableOut = `local/workspace/reports/RUNTIME_PROOF_RELEASE_GATE_${profile.toUpperCase()}_CURRENT.md`;
    const adapterChaosOut = `core/local/artifacts/adapter_runtime_chaos_gate_${profile}_current.json`;
    const boundednessInspectOut = `core/local/artifacts/runtime_boundedness_inspect_${profile}_current.json`;
    const boundednessInspectMarkdownOut = `local/workspace/reports/RUNTIME_BOUNDEDNESS_INSPECT_${profile.toUpperCase()}_CURRENT.md`;

    const harnessExit = runHarness([
      '--strict=1',
      `--profile=${profile}`,
      `--proof-track=${args.proofTrack}`,
      `--out=${harnessOut}`,
      `--metrics-out=${harnessMetricsOut}`,
    ]);
    const boundednessInspectExit = runBoundednessInspect([
      '--strict=0',
      `--profile=${profile}`,
      `--metrics=${harnessMetricsOut}`,
      `--out=${boundednessInspectOut}`,
      `--out-markdown=${boundednessInspectMarkdownOut}`,
    ]);
    const adapterChaosExit = runAdapterChaosGate([
      '--strict=1',
      `--profile=${profile}`,
      `--out=${adapterChaosOut}`,
    ]);
    const gateExit = runReleaseGate([
      '--strict=1',
      `--profile=${profile}`,
      `--proof-track=${args.proofTrack}`,
      `--harness=${harnessOut}`,
      `--adapter-chaos=${adapterChaosOut}`,
      '--policy=tests/tooling/config/release_gates.yaml',
      `--out=${gateOut}`,
      `--metrics-out=${gateMetricsOut}`,
      `--table-out=${gateTableOut}`,
    ]);

    const harnessPayload = readJsonBestEffort(harnessOut);
    const gatePayload = readJsonBestEffort(gateOut);
    const boundednessInspectPayload = readJsonBestEffort(boundednessInspectOut);
    const empiricalSamplePoints = Number(harnessPayload?.proof_tracks?.empirical?.sample_points || 0);
    const empiricalRequired = args.proofTrack === 'empirical' || args.proofTrack === 'dual';
    const empiricalGateOk = !empiricalRequired || empiricalSamplePoints > 0;
    const boundednessScenario = Array.isArray(harnessPayload?.scenarios)
      ? harnessPayload.scenarios.find((row: any) => cleanText(row?.id || '', 80) === 'boundedness_72h')
      : null;
    const soakSource = Array.isArray(harnessPayload?.proof_tracks?.empirical?.sources)
      ? harnessPayload.proof_tracks.empirical.sources.find(
          (row: any) => cleanText(row?.id || '', 120) === 'ops_ipc_bridge_stability_soak',
        )
      : null;

    return {
      profile,
      harnessExit,
      boundednessInspectExit,
      gateExit,
      adapterChaosExit,
      empiricalGateOk,
      empiricalSamplePoints,
      harnessOut,
      harnessMetricsOut,
      boundednessInspectOut,
      boundednessInspectMarkdownOut,
      gateOut,
      gateMetricsOut,
      gateTableOut,
      adapterChaosOut,
      harnessPayload,
      gatePayload,
      boundednessInspectPayload,
      boundednessScenario,
      soakSource,
      ok:
        harnessExit === 0 &&
        gateExit === 0 &&
        adapterChaosExit === 0 &&
        empiricalGateOk &&
        boundednessInspectExit === 0,
    };
  });

  const boundednessOut = 'core/local/artifacts/runtime_boundedness_72h_evidence_current.json';
  const boundednessProfilesOut = 'core/local/artifacts/runtime_boundedness_profiles_current.json';
  const multiDaySoakOut = 'core/local/artifacts/runtime_multi_day_soak_evidence_current.json';
  const syntheticCanaryOut = 'core/local/artifacts/runtime_proof_synthetic_canary_current.json';
  const empiricalReleaseEvidenceOut = 'core/local/artifacts/runtime_proof_empirical_release_evidence_current.json';
  const empiricalReleaseEvidenceMarkdownOut =
    'local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_RELEASE_EVIDENCE_CURRENT.md';
  const empiricalTrendsOut = 'core/local/artifacts/runtime_proof_empirical_trends_current.json';
  const empiricalTrendsMarkdownOut =
    'local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_TRENDS_CURRENT.md';
  const empiricalProfileCoverageOut =
    'core/local/artifacts/runtime_proof_empirical_profile_coverage_current.json';
  const empiricalProfileCoverageMarkdownOut =
    'local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_PROFILE_COVERAGE_CURRENT.md';
  const empiricalSourceMatrixOut =
    'core/local/artifacts/runtime_proof_empirical_source_matrix_current.json';
  const empiricalSourceMatrixMarkdownOut =
    'local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_SOURCE_MATRIX_CURRENT.md';
  const empiricalProfileGateOut =
    'core/local/artifacts/runtime_proof_empirical_profile_gate_current.json';
  const empiricalProfileGateMarkdownOut =
    'local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_PROFILE_GATE_CURRENT.md';
  const empiricalProfileGateFailuresOut =
    'core/local/artifacts/runtime_proof_empirical_profile_gate_failures_current.json';
  const empiricalProfileGateFailuresMarkdownOut =
    'local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_PROFILE_GATE_FAILURES_CURRENT.md';
  const empiricalProfileReadinessOut =
    'core/local/artifacts/runtime_proof_empirical_profile_readiness_current.json';
  const empiricalProfileReadinessMarkdownOut =
    'local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_PROFILE_READINESS_CURRENT.md';
  const proofChecksumsOut = 'core/local/artifacts/release_proof_checksums_current.json';

  const boundednessEvidence = {
    ok: profileRuns.every((row) => !!row.boundednessScenario && row.boundednessScenario.ok === true),
    type: 'runtime_boundedness_72h_evidence',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    proof_track: args.proofTrack,
    profiles: profileRuns.map((row) => ({
      profile: row.profile,
      scenario_present: !!row.boundednessScenario,
      scenario_ok: row.boundednessScenario?.ok === true,
      metrics: row.boundednessScenario?.metrics || {},
      source_artifact: row.harnessOut,
    })),
  };
  writeJsonArtifact(boundednessOut, boundednessEvidence);

  const boundednessProfilesEvidence = {
    ok: profileRuns.every((row) => row.boundednessInspectExit === 0 && row.boundednessInspectPayload?.ok === true),
    type: 'runtime_boundedness_profiles',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    proof_track: args.proofTrack,
    profiles: profileRuns.map((row) => ({
      profile: row.profile,
      ok: row.boundednessInspectPayload?.ok === true,
      summary: row.boundednessInspectPayload?.summary || {},
      rows: Array.isArray(row.boundednessInspectPayload?.rows) ? row.boundednessInspectPayload.rows : [],
      source_artifact: row.boundednessInspectOut,
      source_markdown: row.boundednessInspectMarkdownOut,
    })),
  };
  writeJsonArtifact(boundednessProfilesOut, boundednessProfilesEvidence);

  const multiDaySoakEvidence = {
    ok: profileRuns.every((row) => row.empiricalSamplePoints > 0 && (row.soakSource?.loaded === true || row.soakSource?.sample_points > 0)),
    type: 'runtime_multi_day_soak_evidence',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    proof_track: args.proofTrack,
    profiles: profileRuns.map((row) => ({
      profile: row.profile,
      empirical_sample_points: row.empiricalSamplePoints,
      soak_source_loaded: row.soakSource?.loaded === true,
      soak_source_sample_points: Number(row.soakSource?.sample_points || 0),
      soak_source_detail: cleanText(row.soakSource?.detail || 'missing', 200),
      source_artifact: row.harnessOut,
    })),
  };
  writeJsonArtifact(multiDaySoakOut, multiDaySoakEvidence);

  const syntheticCanaryEvidence = {
    ok: profileRuns.every((row) => row.harnessExit === 0),
    type: 'runtime_proof_synthetic_canary',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    proof_track: args.proofTrack,
    profiles: profileRuns.map((row) => ({
      profile: row.profile,
      sample_points: Number(row.harnessPayload?.proof_tracks?.synthetic?.sample_points || 0),
      metrics: row.harnessPayload?.proof_tracks?.synthetic?.metrics || {},
      source_artifact: row.harnessOut,
    })),
  };
  writeJsonArtifact(syntheticCanaryOut, syntheticCanaryEvidence);

  const empiricalReleaseEvidence = {
    ok: profileRuns.every((row) => row.empiricalSamplePoints > 0),
    type: 'runtime_proof_empirical_release_evidence',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    proof_track: args.proofTrack,
    profiles: profileRuns.map((row) => ({
      profile: row.profile,
      sample_points: row.empiricalSamplePoints,
      metrics: row.harnessPayload?.proof_tracks?.empirical?.metrics || {},
      provided_keys: row.harnessPayload?.proof_tracks?.empirical?.provided_keys || [],
      sources: row.harnessPayload?.proof_tracks?.empirical?.sources || [],
      release_gate_ok: row.gateExit === 0,
      source_artifact: row.harnessOut,
    })),
  };
  writeJsonArtifact(empiricalReleaseEvidenceOut, empiricalReleaseEvidence);
  ensureParentDir(empiricalReleaseEvidenceMarkdownOut);
  fs.writeFileSync(
    empiricalReleaseEvidenceMarkdownOut,
    renderEmpiricalReleaseEvidenceMarkdown(empiricalReleaseEvidence),
    'utf8',
  );

  const empiricalSnapshot = {
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    proof_track: args.proofTrack,
    profiles: profileRuns.map((row) => ({
      profile: row.profile,
      sample_points: row.empiricalSamplePoints,
      metrics: EMPIRICAL_TREND_METRIC_KEYS.reduce(
        (acc, key) => {
          acc[key] = safeNumber(row.harnessPayload?.proof_tracks?.empirical?.metrics?.[key], 0);
          return acc;
        },
        {} as Record<EmpiricalTrendMetricKey, number>,
      ),
      source_artifact: row.harnessOut,
    })),
  };
  appendJsonLine(args.empiricalHistoryPath, empiricalSnapshot);
  const empiricalHistory = readJsonLinesBestEffort(args.empiricalHistoryPath);
  const previousSnapshot = empiricalHistory.length > 1 ? empiricalHistory[empiricalHistory.length - 2] : null;
  const previousByProfile = new Map<string, any>(
    Array.isArray(previousSnapshot?.profiles)
      ? previousSnapshot.profiles.map((row: any) => [cleanText(row?.profile || '', 40), row])
      : [],
  );
  const empiricalTrends = {
    ok: profileRuns.every((row) => row.empiricalSamplePoints > 0),
    type: 'runtime_proof_empirical_trends',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    history_samples: empiricalHistory.length,
    baseline_available: !!previousSnapshot,
    proof_track: args.proofTrack,
    profiles: empiricalSnapshot.profiles.map((row) => {
      const previous = previousByProfile.get(row.profile);
      const previousSamplePoints = safeNumber(previous?.sample_points, 0);
      return {
        profile: row.profile,
        sample_points: row.sample_points,
        sample_points_delta: row.sample_points - previousSamplePoints,
        metric_deltas: EMPIRICAL_TREND_METRIC_KEYS.reduce(
          (acc, key) => {
            const currentValue = safeNumber(row.metrics?.[key], 0);
            const previousValue = safeNumber(previous?.metrics?.[key], 0);
            acc[key] = {
              current: currentValue,
              previous: previousValue,
              delta: currentValue - previousValue,
            };
            return acc;
          },
          {} as Record<EmpiricalTrendMetricKey, { current: number; previous: number; delta: number }>,
        ),
      };
    }),
    history_path: args.empiricalHistoryPath,
  };
  writeJsonArtifact(empiricalTrendsOut, empiricalTrends);
  ensureParentDir(empiricalTrendsMarkdownOut);
  fs.writeFileSync(
    empiricalTrendsMarkdownOut,
    renderEmpiricalTrendsMarkdown(empiricalTrends),
    'utf8',
  );

  const empiricalProfileCoverageRows = profileRuns.map((row) => {
      const requiredMinSamplePoints = safeNumber(
        row.gatePayload?.profile_requirements?.empirical_min_sample_points,
        safeNumber(
          row.gatePayload?.effective_policy?.empirical_min_sample_points,
          safeNumber(row.gatePayload?.empirical_min_sample_points, 0),
        ),
      );
      const providedKeys = toStringArray(row.harnessPayload?.proof_tracks?.empirical?.provided_keys, 120);
      const sources = Array.isArray(row.harnessPayload?.proof_tracks?.empirical?.sources)
        ? row.harnessPayload.proof_tracks.empirical.sources
        : [];
      const missingRequiredSourceIds = toStringArray(
        row.gatePayload?.metrics?.proof_track_empirical_required_sources_missing,
        120,
      );
      const missingRequiredMetricKeys = toStringArray(
        row.gatePayload?.metrics?.proof_track_empirical_required_metrics_missing,
        120,
      );
      const nonPositiveRequiredMetricKeys = toStringArray(
        row.gatePayload?.metrics?.proof_track_empirical_required_positive_metrics_missing,
        120,
      );
      const samplePointsOk = row.empiricalSamplePoints >= requiredMinSamplePoints;
      const requiredSourcesOk = missingRequiredSourceIds.length === 0;
      const requiredMetricsOk = missingRequiredMetricKeys.length === 0;
      const requiredPositiveMetricsOk = nonPositiveRequiredMetricKeys.length === 0;
      const empiricalReleaseGateChecksFailed = Array.isArray(row.gatePayload?.checks)
        ? row.gatePayload.checks
            .filter((check: any) => {
              const id = cleanText(String(check?.id || ''), 120);
              const pass = check?.pass === true;
              return id.startsWith('proof_track_empirical_') && !pass;
            })
            .map((check: any) => cleanText(String(check?.id || ''), 120))
            .filter(Boolean)
        : [];
      const empiricalReleaseGateExecutionOk =
        row.gateExit === 0 &&
        row.gatePayload?.ok === true &&
        empiricalReleaseGateChecksFailed.length === 0;
      const releaseGateReasons: string[] = [];
      if (row.empiricalSamplePoints <= 0) {
        releaseGateReasons.push('empirical_sample_points_missing');
      }
      if (!samplePointsOk) {
        releaseGateReasons.push('empirical_sample_points_below_profile_minimum');
      }
      if (providedKeys.length === 0) {
        releaseGateReasons.push('empirical_provided_keys_missing');
      }
      if (sources.length === 0) {
        releaseGateReasons.push('empirical_sources_missing');
      }
      if (!requiredSourcesOk) {
        releaseGateReasons.push('empirical_required_sources_missing');
      }
      if (!requiredMetricsOk) {
        releaseGateReasons.push('empirical_required_metrics_missing');
      }
      if (!requiredPositiveMetricsOk) {
        releaseGateReasons.push('empirical_required_positive_metrics_non_positive');
      }
      if (!empiricalReleaseGateExecutionOk) {
        releaseGateReasons.push('empirical_release_gate_checks_failed');
      }
      const empiricalMetrics = row.harnessPayload?.proof_tracks?.empirical?.metrics || {};
      const sourceRows = sources.map((source: any) => ({
        id: cleanText(source?.id || '', 80),
        loaded: source?.loaded === true,
        sample_points: safeNumber(source?.sample_points, 0),
        detail: cleanText(source?.detail || '', 200),
        artifact_path: cleanText(
          source?.artifact_path || source?.artifact || source?.path || '',
          260,
        ),
      }));
      const sourceRowById = new Map<string, any>(
        sourceRows.map((sourceRow: any) => [cleanText(sourceRow.id, 80), sourceRow]),
      );
      const requiredSourceIds = uniqueStringValues(
        sourceRows.map((sourceRow: any) => cleanText(sourceRow.id, 80)).concat(missingRequiredSourceIds),
      );
      const missingSourceSet = new Set(missingRequiredSourceIds);
      const requiredSourceRows = requiredSourceIds.map((id) => {
        const rowSource = sourceRowById.get(id) || {};
        const loaded = rowSource.loaded === true;
        const samplePoints = safeNumber(rowSource.sample_points, 0);
        const requiredMissing = missingSourceSet.has(id);
        const requiredSatisfied = !requiredMissing && (loaded || samplePoints > 0);
        return {
          id,
          loaded,
          sample_points: samplePoints,
          detail: cleanText(rowSource.detail || '', 200),
          artifact_path: cleanText(rowSource.artifact_path || '', 260),
          required_missing: requiredMissing,
          required_satisfied: requiredSatisfied,
        };
      });
      const metricKeys = uniqueStringValues(
        providedKeys.concat(missingRequiredMetricKeys).concat(nonPositiveRequiredMetricKeys),
      );
      const missingMetricSet = new Set(missingRequiredMetricKeys);
      const nonPositiveMetricSet = new Set(nonPositiveRequiredMetricKeys);
      const requiredMetricRows = metricKeys.map((key) => {
        const value = safeNumber(empiricalMetrics?.[key], 0);
        const requiredMissing = missingMetricSet.has(key);
        const requiredNonPositive = nonPositiveMetricSet.has(key);
        const requiredSatisfied = !requiredMissing && !requiredNonPositive;
        return {
          key,
          value,
          provided: providedKeys.includes(key),
          required_missing: requiredMissing,
          required_non_positive: requiredNonPositive,
          required_satisfied: requiredSatisfied,
        };
      });
      return {
        profile: row.profile,
        empirical_sample_points: row.empiricalSamplePoints,
        empirical_min_sample_points_required: requiredMinSamplePoints,
        empirical_sample_points_ok: samplePointsOk,
        empirical_provided_keys_count: providedKeys.length,
        empirical_provided_keys: providedKeys,
        empirical_sources_count: sources.length,
        empirical_required_sources_missing: missingRequiredSourceIds,
        empirical_required_metrics_missing: missingRequiredMetricKeys,
        empirical_required_positive_metrics_missing: nonPositiveRequiredMetricKeys,
        empirical_required_sources_ok: requiredSourcesOk,
        empirical_required_metrics_ok: requiredMetricsOk,
        empirical_required_positive_metrics_ok: requiredPositiveMetricsOk,
        empirical_release_gate_execution_ok: empiricalReleaseGateExecutionOk,
        empirical_release_gate_checks_failed: empiricalReleaseGateChecksFailed,
        empirical_release_gate_reasons: releaseGateReasons,
        empirical_release_gate_pass: releaseGateReasons.length === 0,
        empirical_source_rows: sourceRows,
        empirical_required_source_rows: requiredSourceRows,
        empirical_required_metric_rows: requiredMetricRows,
        source_artifact: row.harnessOut,
        release_gate_artifact: row.gateOut,
      };
    });

  const empiricalProfileCoverage = {
    ok: empiricalProfileCoverageRows.every(
      (row) =>
        row.empirical_sample_points > 0 &&
        row.empirical_sample_points_ok &&
        row.empirical_provided_keys_count > 0 &&
        row.empirical_sources_count > 0 &&
        row.empirical_required_sources_ok &&
        row.empirical_required_metrics_ok &&
        row.empirical_required_positive_metrics_ok,
    ),
    type: 'runtime_proof_empirical_profile_coverage',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    proof_track: args.proofTrack,
    profiles: empiricalProfileCoverageRows,
  };
  writeJsonArtifact(empiricalProfileCoverageOut, empiricalProfileCoverage);
  ensureParentDir(empiricalProfileCoverageMarkdownOut);
  fs.writeFileSync(
    empiricalProfileCoverageMarkdownOut,
    renderEmpiricalProfileCoverageMarkdown(empiricalProfileCoverage),
    'utf8',
  );

  const empiricalSourceMatrix = {
    ok: empiricalProfileCoverageRows.every(
      (row) =>
        row.empirical_sample_points_ok &&
        row.empirical_required_sources_ok &&
        row.empirical_required_metrics_ok &&
        row.empirical_required_positive_metrics_ok,
    ),
    type: 'runtime_proof_empirical_source_matrix',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    proof_track: args.proofTrack,
    profiles: empiricalProfileCoverageRows,
  };
  writeJsonArtifact(empiricalSourceMatrixOut, empiricalSourceMatrix);
  ensureParentDir(empiricalSourceMatrixMarkdownOut);
  fs.writeFileSync(
    empiricalSourceMatrixMarkdownOut,
    renderEmpiricalSourceMatrixMarkdown(empiricalSourceMatrix),
    'utf8',
  );

  const failingProfiles = empiricalProfileCoverageRows
    .filter((row) => row.empirical_release_gate_pass !== true)
    .map((row) => cleanText(row.profile || '', 40))
    .filter(Boolean);
  const reasonBuckets = empiricalProfileCoverageRows.reduce((acc, row) => {
    const reasons = toStringArray(row.empirical_release_gate_reasons, 160);
    for (const reason of reasons) {
      const key = cleanText(reason, 120);
      if (!key) continue;
      const next = safeNumber(acc[key], 0) + 1;
      acc[key] = next;
    }
    return acc;
  }, {} as Record<string, number>);
  const empiricalProfileGate = {
    ok: failingProfiles.length === 0,
    type: 'runtime_proof_empirical_profile_gate',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    proof_track: args.proofTrack,
    summary: {
      profiles_total: empiricalProfileCoverageRows.length,
      profiles_passed: empiricalProfileCoverageRows.length - failingProfiles.length,
      profiles_failed: failingProfiles.length,
      failing_profiles: failingProfiles,
      reason_buckets: reasonBuckets,
    },
    profiles: empiricalProfileCoverageRows.map((row) => ({
      profile: row.profile,
      empirical_release_gate_pass: row.empirical_release_gate_pass === true,
      empirical_release_gate_execution_ok: row.empirical_release_gate_execution_ok === true,
      empirical_release_gate_checks_failed: toStringArray(
        row.empirical_release_gate_checks_failed,
        160,
      ),
      empirical_release_gate_reasons: toStringArray(row.empirical_release_gate_reasons, 160),
      empirical_sample_points: safeNumber(row.empirical_sample_points, 0),
      empirical_min_sample_points_required: safeNumber(
        row.empirical_min_sample_points_required,
        0,
      ),
      empirical_sample_points_ok: row.empirical_sample_points_ok === true,
      empirical_required_sources_ok: row.empirical_required_sources_ok === true,
      empirical_required_metrics_ok: row.empirical_required_metrics_ok === true,
      empirical_required_positive_metrics_ok:
        row.empirical_required_positive_metrics_ok === true,
      source_artifact: cleanText(row.source_artifact || '', 260),
      release_gate_artifact: cleanText(row.release_gate_artifact || '', 260),
    })),
  };
  writeJsonArtifact(empiricalProfileGateOut, empiricalProfileGate);
  ensureParentDir(empiricalProfileGateMarkdownOut);
  fs.writeFileSync(
    empiricalProfileGateMarkdownOut,
    renderEmpiricalProfileGateMarkdown(empiricalProfileGate),
    'utf8',
  );
  const empiricalProfileGateFailures = {
    ok: true,
    type: 'runtime_proof_empirical_profile_gate_failures',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    proof_track: args.proofTrack,
    has_failures: failingProfiles.length > 0,
    failing_profiles_count: failingProfiles.length,
    failures: empiricalProfileCoverageRows
      .filter((row) => row.empirical_release_gate_pass !== true)
      .map((row) => ({
        profile: row.profile,
        reasons: toStringArray(row.empirical_release_gate_reasons, 160),
        release_gate_checks_failed: toStringArray(
          row.empirical_release_gate_checks_failed,
          160,
        ),
        required_sources_missing: toStringArray(
          row.empirical_required_sources_missing,
          120,
        ),
        required_metrics_missing: toStringArray(
          row.empirical_required_metrics_missing,
          120,
        ),
        required_positive_metrics_missing: toStringArray(
          row.empirical_required_positive_metrics_missing,
          120,
        ),
        empirical_sample_points: safeNumber(row.empirical_sample_points, 0),
        empirical_min_sample_points_required: safeNumber(
          row.empirical_min_sample_points_required,
          0,
        ),
        source_artifact: cleanText(row.source_artifact || '', 260),
        release_gate_artifact: cleanText(row.release_gate_artifact || '', 260),
      })),
  };
  writeJsonArtifact(empiricalProfileGateFailuresOut, empiricalProfileGateFailures);
  ensureParentDir(empiricalProfileGateFailuresMarkdownOut);
  fs.writeFileSync(
    empiricalProfileGateFailuresMarkdownOut,
    renderEmpiricalProfileGateFailuresMarkdown(empiricalProfileGateFailures),
    'utf8',
  );

  const severeReasonIds = new Set([
    'empirical_sample_points_missing',
    'empirical_sample_points_below_profile_minimum',
    'empirical_required_sources_missing',
    'empirical_required_metrics_missing',
    'empirical_required_positive_metrics_non_positive',
    'empirical_release_gate_checks_failed',
  ]);
  const empiricalProfileReadinessRows = empiricalProfileCoverageRows.map((row) => {
    const reasons = toStringArray(row.empirical_release_gate_reasons, 160);
    const severeReasons = reasons.filter((reason) => severeReasonIds.has(cleanText(reason, 120)));
    let readinessScore = 100;
    if (safeNumber(row.empirical_sample_points, 0) <= 0) readinessScore -= 45;
    if (row.empirical_sample_points_ok !== true) readinessScore -= 20;
    if (safeNumber(row.empirical_provided_keys_count, 0) <= 0) readinessScore -= 10;
    if (safeNumber(row.empirical_sources_count, 0) <= 0) readinessScore -= 10;
    if (row.empirical_required_sources_ok !== true) readinessScore -= 15;
    if (row.empirical_required_metrics_ok !== true) readinessScore -= 15;
    if (row.empirical_required_positive_metrics_ok !== true) readinessScore -= 10;
    if (row.empirical_release_gate_execution_ok !== true) readinessScore -= 15;
    readinessScore = Math.max(0, Math.min(100, readinessScore));
    const readinessClass =
      row.empirical_release_gate_pass === true
        ? 'release_ready'
        : severeReasons.length > 0
          ? 'blocked'
          : 'degraded';
    return {
      profile: cleanText(row.profile || '', 40),
      readiness_class: readinessClass,
      readiness_score: readinessScore,
      reasons,
      severe_reasons: severeReasons,
      release_gate_checks_failed: toStringArray(row.empirical_release_gate_checks_failed, 160),
      source_artifact: cleanText(row.source_artifact || '', 260),
      release_gate_artifact: cleanText(row.release_gate_artifact || '', 260),
    };
  });
  const readinessScoreTotal = empiricalProfileReadinessRows.reduce(
    (acc, row) => acc + safeNumber(row.readiness_score, 0),
    0,
  );
  const empiricalProfileReadiness = {
    ok: empiricalProfileReadinessRows.every((row) => row.readiness_class === 'release_ready'),
    type: 'runtime_proof_empirical_profile_readiness',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    proof_track: args.proofTrack,
    summary: {
      profiles_total: empiricalProfileReadinessRows.length,
      release_ready_profiles: empiricalProfileReadinessRows.filter(
        (row) => row.readiness_class === 'release_ready',
      ).length,
      degraded_profiles: empiricalProfileReadinessRows.filter(
        (row) => row.readiness_class === 'degraded',
      ).length,
      blocked_profiles: empiricalProfileReadinessRows.filter(
        (row) => row.readiness_class === 'blocked',
      ).length,
      readiness_score_avg:
        empiricalProfileReadinessRows.length > 0
          ? Math.round((readinessScoreTotal / empiricalProfileReadinessRows.length) * 100) / 100
          : 0,
    },
    profiles: empiricalProfileReadinessRows,
  };
  writeJsonArtifact(empiricalProfileReadinessOut, empiricalProfileReadiness);
  ensureParentDir(empiricalProfileReadinessMarkdownOut);
  fs.writeFileSync(
    empiricalProfileReadinessMarkdownOut,
    renderEmpiricalProfileReadinessMarkdown(empiricalProfileReadiness),
    'utf8',
  );

  const checksumRows = profileRuns
    .flatMap((row) => [
      row.harnessOut,
      row.harnessMetricsOut,
      row.boundednessInspectOut,
      row.boundednessInspectMarkdownOut,
      row.gateOut,
      row.gateMetricsOut,
      row.adapterChaosOut,
      row.gateTableOut,
    ])
    .concat([
      boundednessOut,
      boundednessProfilesOut,
      multiDaySoakOut,
      syntheticCanaryOut,
      empiricalReleaseEvidenceOut,
      empiricalReleaseEvidenceMarkdownOut,
      empiricalTrendsOut,
      empiricalTrendsMarkdownOut,
      empiricalProfileCoverageOut,
      empiricalProfileCoverageMarkdownOut,
      empiricalSourceMatrixOut,
      empiricalSourceMatrixMarkdownOut,
      empiricalProfileGateOut,
      empiricalProfileGateMarkdownOut,
      empiricalProfileGateFailuresOut,
      empiricalProfileGateFailuresMarkdownOut,
      empiricalProfileReadinessOut,
      empiricalProfileReadinessMarkdownOut,
      args.empiricalHistoryPath,
    ])
    .map((artifactPath) => ({
      path: artifactPath,
      exists: fs.existsSync(artifactPath),
      sha256: sha256File(artifactPath),
    }));
  const proofChecksums = {
    ok: checksumRows.every((row) => row.exists && row.sha256.length > 0),
    type: 'release_proof_checksums',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    checksums: checksumRows,
  };
  writeJsonArtifact(proofChecksumsOut, proofChecksums);

  const ok =
    profileRuns.every((row) => row.ok) &&
    boundednessEvidence.ok &&
    boundednessProfilesEvidence.ok &&
    multiDaySoakEvidence.ok &&
    empiricalProfileCoverage.ok &&
    empiricalProfileGate.ok &&
    empiricalProfileReadiness.ok &&
    proofChecksums.ok;
  const artifactPaths = profileRuns
    .flatMap((row) => [
      row.harnessOut,
      row.harnessMetricsOut,
      row.boundednessInspectOut,
      row.boundednessInspectMarkdownOut,
      row.gateOut,
      row.gateMetricsOut,
      row.gateTableOut,
      row.adapterChaosOut,
    ])
    .concat([
      boundednessOut,
      boundednessProfilesOut,
      multiDaySoakOut,
      syntheticCanaryOut,
      empiricalReleaseEvidenceOut,
      empiricalReleaseEvidenceMarkdownOut,
      empiricalTrendsOut,
      empiricalTrendsMarkdownOut,
      empiricalProfileCoverageOut,
      empiricalProfileCoverageMarkdownOut,
      empiricalSourceMatrixOut,
      empiricalSourceMatrixMarkdownOut,
      empiricalProfileGateOut,
      empiricalProfileGateMarkdownOut,
      empiricalProfileGateFailuresOut,
      empiricalProfileGateFailuresMarkdownOut,
      empiricalProfileReadinessOut,
      empiricalProfileReadinessMarkdownOut,
      args.empiricalHistoryPath,
      proofChecksumsOut,
    ]);

  const report = {
    ok,
    type: 'runtime_proof_verify',
    profile: args.profile,
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    summary: {
      pass: ok,
      proof_track: args.proofTrack,
      profile_count: profileRuns.length,
      profiles_passed: profileRuns.filter((row) => row.ok).length,
      boundedness_inspect_exit_max: Math.max(...profileRuns.map((row) => row.boundednessInspectExit)),
    },
    profile_runs: profileRuns.map((row) => ({
      profile: row.profile,
      ok: row.ok,
      harness_exit: row.harnessExit,
      release_gate_exit: row.gateExit,
      adapter_runtime_chaos_exit: row.adapterChaosExit,
      empirical_sample_points: row.empiricalSamplePoints,
      empirical_sample_points_ok: row.empiricalGateOk,
      artifact_paths: [
        row.harnessOut,
        row.harnessMetricsOut,
        row.boundednessInspectOut,
        row.boundednessInspectMarkdownOut,
        row.gateOut,
        row.gateMetricsOut,
        row.gateTableOut,
        row.adapterChaosOut,
      ],
    })),
    evidence: {
      boundedness_72h: boundednessOut,
      boundedness_profiles: boundednessProfilesOut,
      multi_day_soak: multiDaySoakOut,
      synthetic_canary: syntheticCanaryOut,
      empirical_release_evidence: empiricalReleaseEvidenceOut,
      empirical_release_evidence_markdown: empiricalReleaseEvidenceMarkdownOut,
      empirical_trends: empiricalTrendsOut,
      empirical_trends_markdown: empiricalTrendsMarkdownOut,
      empirical_profile_coverage: empiricalProfileCoverageOut,
      empirical_profile_coverage_markdown: empiricalProfileCoverageMarkdownOut,
      empirical_source_matrix: empiricalSourceMatrixOut,
      empirical_source_matrix_markdown: empiricalSourceMatrixMarkdownOut,
      empirical_profile_gate: empiricalProfileGateOut,
      empirical_profile_gate_markdown: empiricalProfileGateMarkdownOut,
      empirical_profile_gate_failures: empiricalProfileGateFailuresOut,
      empirical_profile_gate_failures_markdown: empiricalProfileGateFailuresMarkdownOut,
      empirical_profile_readiness: empiricalProfileReadinessOut,
      empirical_profile_readiness_markdown: empiricalProfileReadinessMarkdownOut,
      empirical_history: args.empiricalHistoryPath,
      proof_checksums: proofChecksumsOut,
    },
    artifact_paths: artifactPaths,
    failures: [
      ...profileRuns.flatMap((row) => [
        ...(row.boundednessInspectExit === 0
          ? []
          : [{ id: 'runtime_boundedness_inspect_failed', detail: `profile=${row.profile};exit_code=${row.boundednessInspectExit}` }]),
        ...(row.boundednessInspectPayload?.ok === true
          ? []
          : [{ id: 'runtime_boundedness_inspect_report_not_ok', detail: `profile=${row.profile};artifact=${row.boundednessInspectOut}` }]),
        ...(row.harnessExit === 0
          ? []
          : [{ id: 'runtime_proof_harness_failed', detail: `profile=${row.profile};exit_code=${row.harnessExit}` }]),
        ...(row.gateExit === 0
          ? []
          : [{ id: 'runtime_proof_release_gate_failed', detail: `profile=${row.profile};exit_code=${row.gateExit}` }]),
        ...(row.adapterChaosExit === 0
          ? []
          : [{ id: 'adapter_runtime_chaos_gate_failed', detail: `profile=${row.profile};exit_code=${row.adapterChaosExit}` }]),
        ...(row.empiricalGateOk
          ? []
          : [{ id: 'runtime_proof_empirical_sample_points_missing', detail: `profile=${row.profile};sample_points=${row.empiricalSamplePoints}` }]),
      ]),
      ...(boundednessEvidence.ok ? [] : [{ id: 'runtime_boundedness_72h_evidence_incomplete', detail: boundednessOut }]),
      ...(boundednessProfilesEvidence.ok
        ? []
        : [{ id: 'runtime_boundedness_profiles_evidence_incomplete', detail: boundednessProfilesOut }]),
      ...(multiDaySoakEvidence.ok ? [] : [{ id: 'runtime_multi_day_soak_evidence_incomplete', detail: multiDaySoakOut }]),
      ...(syntheticCanaryEvidence.ok ? [] : [{ id: 'runtime_proof_synthetic_canary_incomplete', detail: syntheticCanaryOut }]),
      ...(empiricalReleaseEvidence.ok
        ? []
        : [{ id: 'runtime_proof_empirical_release_evidence_incomplete', detail: empiricalReleaseEvidenceOut }]),
      ...(empiricalTrends.ok ? [] : [{ id: 'runtime_proof_empirical_trends_incomplete', detail: empiricalTrendsOut }]),
      ...(empiricalProfileCoverage.ok
        ? []
        : [{ id: 'runtime_proof_empirical_profile_coverage_incomplete', detail: empiricalProfileCoverageOut }]),
      ...(empiricalSourceMatrix.ok
        ? []
        : [{ id: 'runtime_proof_empirical_source_matrix_incomplete', detail: empiricalSourceMatrixOut }]),
      ...(empiricalProfileGate.ok
        ? []
        : [{ id: 'runtime_proof_empirical_profile_gate_incomplete', detail: empiricalProfileGateOut }]),
      ...(empiricalProfileReadiness.ok
        ? []
        : [{ id: 'runtime_proof_empirical_profile_readiness_incomplete', detail: empiricalProfileReadinessOut }]),
      ...(proofChecksums.ok ? [] : [{ id: 'release_proof_checksums_incomplete', detail: proofChecksumsOut }]),
    ],
  };

  return emitStructuredResult(report, {
    outPath: args.outPath,
    strict: args.strict,
    ok: report.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
