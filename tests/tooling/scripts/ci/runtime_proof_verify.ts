#!/usr/bin/env tsx

import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeJsonArtifact } from '../../lib/result.ts';
import { run as runHarness } from './runtime_proof_harness.ts';
import { run as runReleaseGate } from './runtime_proof_release_gate.ts';
import { run as runAdapterChaosGate } from './gateway_runtime_chaos_gate.ts';
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

const RELEASE_GATE_QUALITY_PATHS = [
  'artifacts/web_tooling_context_soak_report_latest.json',
  'artifacts/workspace_tooling_context_soak_report_latest.json',
];

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

function isCanonicalArtifactToken(value: unknown): boolean {
  const token = cleanText(value || '', 500);
  if (!token) return false;
  if (path.isAbsolute(token)) return false;
  if (token.includes('\\')) return false;
  if (token.includes('..')) return false;
  if (token.includes('//')) return false;
  if (/\s/.test(token)) return false;
  return true;
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
      `  - empirical_profile_minimum_configured: ${
        profile?.empirical_profile_minimum_configured === true ? 'true' : 'false'
      }`,
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

function renderEmpiricalMinimumContractMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Runtime Proof Empirical Minimum Configuration Contract (Current)');
  lines.push('');
  lines.push(`Generated: ${cleanText(payload?.generated_at || '', 80)}`);
  lines.push(`Revision: ${cleanText(payload?.revision || '', 120)}`);
  lines.push(`Proof track: ${cleanText(payload?.proof_track || '', 32)}`);
  lines.push(`Pass: ${payload?.ok === true ? 'true' : 'false'}`);
  const summary = payload?.summary || {};
  lines.push(`Profiles total: ${safeNumber(summary?.profiles_total, 0)}`);
  lines.push(`Configured profiles: ${safeNumber(summary?.configured_profiles, 0)}`);
  lines.push(`Missing profiles: ${safeNumber(summary?.missing_profiles, 0)}`);
  lines.push(`Minimum floor: ${safeNumber(summary?.min_floor, 0)}`);
  lines.push(`Maximum floor: ${safeNumber(summary?.max_floor, 0)}`);
  const missing = toStringArray(summary?.missing_profile_ids, 80);
  lines.push(`Missing profile ids: ${missing.length > 0 ? missing.join(', ') : 'none'}`);
  lines.push('');
  const profiles = Array.isArray(payload?.profiles) ? payload.profiles : [];
  for (const profile of profiles) {
    lines.push(`## ${cleanText(profile?.profile || 'unknown', 40)}`);
    lines.push(
      `- empirical_min_sample_points_required: ${safeNumber(profile?.empirical_min_sample_points_required, 0)}`,
    );
    lines.push(
      `- empirical_profile_minimum_configured: ${
        profile?.empirical_profile_minimum_configured === true ? 'true' : 'false'
      }`,
    );
    lines.push(
      `- empirical_sample_points: ${safeNumber(profile?.empirical_sample_points, 0)}`,
    );
    lines.push(
      `- source_artifact: ${cleanText(profile?.source_artifact || '', 240) || 'n/a'}`,
    );
    lines.push(
      `- release_gate_artifact: ${cleanText(profile?.release_gate_artifact || '', 240) || 'n/a'}`,
    );
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
    const gatewayChaosOut = `core/local/artifacts/gateway_runtime_chaos_gate_${profile}_current.json`;
    const boundednessInspectOut = `core/local/artifacts/runtime_boundedness_inspect_${profile}_current.json`;
    const boundednessInspectMarkdownOut = `local/workspace/reports/RUNTIME_BOUNDEDNESS_INSPECT_${profile.toUpperCase()}_CURRENT.md`;
    const harnessArgs = [
      '--strict=1',
      `--profile=${profile}`,
      `--proof-track=${args.proofTrack}`,
      `--out=${harnessOut}`,
      `--metrics-out=${harnessMetricsOut}`,
    ];

    const harnessPrepExit = runHarness(harnessArgs);
    const boundednessInspectExit = runBoundednessInspect([
      '--strict=0',
      `--profile=${profile}`,
      `--metrics=${harnessMetricsOut}`,
      `--out=${boundednessInspectOut}`,
      `--out-markdown=${boundednessInspectMarkdownOut}`,
    ]);
    const harnessRefreshExit =
      harnessPrepExit === 0 && boundednessInspectExit === 0 ? runHarness(harnessArgs) : harnessPrepExit;
    const harnessExit = harnessPrepExit === 0 && boundednessInspectExit === 0
      ? harnessRefreshExit
      : harnessPrepExit;
    const gatewayChaosExit = runAdapterChaosGate([
      '--strict=1',
      `--profile=${profile}`,
      `--out=${gatewayChaosOut}`,
    ]);
    const gateExit = runReleaseGate([
      '--strict=1',
      `--profile=${profile}`,
      `--proof-track=${args.proofTrack}`,
      `--harness=${harnessOut}`,
      `--gateway-chaos=${gatewayChaosOut}`,
      `--quality-paths=${RELEASE_GATE_QUALITY_PATHS.join(',')}`,
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
      gatewayChaosExit,
      empiricalGateOk,
      empiricalSamplePoints,
      harnessOut,
      harnessMetricsOut,
      boundednessInspectOut,
      boundednessInspectMarkdownOut,
      gateOut,
      gateMetricsOut,
      gateTableOut,
      gatewayChaosOut,
      harnessPayload,
      gatePayload,
      boundednessInspectPayload,
      boundednessScenario,
      soakSource,
      ok:
        harnessExit === 0 &&
        gatewayChaosExit === 0 &&
        empiricalGateOk &&
        (args.proofTrack === 'empirical' ? true : gateExit === 0) &&
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
  const empiricalMinimumContractOut =
    'core/local/artifacts/runtime_proof_empirical_minimum_contract_current.json';
  const empiricalMinimumContractMarkdownOut =
    'local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_MINIMUM_CONTRACT_CURRENT.md';
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
  const trendWindowSize = Math.min(empiricalHistory.length, 7);
  const trendWindow = trendWindowSize > 0 ? empiricalHistory.slice(-trendWindowSize) : [];
  const windowBaselineSnapshot = trendWindow.length > 0 ? trendWindow[0] : null;
  const windowBaselineByProfile = new Map<string, any>(
    Array.isArray(windowBaselineSnapshot?.profiles)
      ? windowBaselineSnapshot.profiles.map((row: any) => [cleanText(row?.profile || '', 40), row])
      : [],
  );
  const empiricalTrends = {
    ok: profileRuns.every((row) => row.empiricalSamplePoints > 0),
    type: 'runtime_proof_empirical_trends',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    history_samples: empiricalHistory.length,
    baseline_available: !!previousSnapshot,
    trend_window_samples: trendWindowSize,
    proof_track: args.proofTrack,
    profiles: empiricalSnapshot.profiles.map((row) => {
      const previous = previousByProfile.get(row.profile);
      const previousSamplePoints = safeNumber(previous?.sample_points, 0);
      const windowBaseline = windowBaselineByProfile.get(row.profile);
      const windowBaselineSamplePoints = safeNumber(windowBaseline?.sample_points, 0);
      const metricDeltas = EMPIRICAL_TREND_METRIC_KEYS.reduce(
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
      );
      const metricWindowDeltas = EMPIRICAL_TREND_METRIC_KEYS.reduce(
        (acc, key) => {
          const currentValue = safeNumber(row.metrics?.[key], 0);
          const previousValue = safeNumber(previous?.metrics?.[key], 0);
          const baselineValue = safeNumber(windowBaseline?.metrics?.[key], 0);
          const deltaFromBaseline = currentValue - baselineValue;
          acc[key] = {
            current: currentValue,
            previous: previousValue,
            baseline: baselineValue,
            delta: currentValue - previousValue,
            delta_from_baseline: deltaFromBaseline,
            direction: deltaFromBaseline > 0 ? 'up' : deltaFromBaseline < 0 ? 'down' : 'flat',
          };
          return acc;
        },
        {} as Record<
          EmpiricalTrendMetricKey,
          {
            current: number;
            previous: number;
            baseline: number;
            delta: number;
            delta_from_baseline: number;
            direction: 'up' | 'down' | 'flat';
          }
        >,
      );
      const metricDirectionCounts = EMPIRICAL_TREND_METRIC_KEYS.reduce(
        (acc, key) => {
          const direction = metricWindowDeltas[key].direction;
          if (direction === 'up') acc.up += 1;
          else if (direction === 'down') acc.down += 1;
          else acc.flat += 1;
          return acc;
        },
        { up: 0, down: 0, flat: 0 },
      );
      return {
        profile: row.profile,
        sample_points: row.sample_points,
        sample_points_delta: row.sample_points - previousSamplePoints,
        sample_points_baseline: windowBaselineSamplePoints,
        sample_points_delta_from_baseline: row.sample_points - windowBaselineSamplePoints,
        metric_deltas: metricDeltas,
        metric_window_deltas: metricWindowDeltas,
        metric_direction_counts: metricDirectionCounts,
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
      const policyFallbackMin =
        row.profile === 'rich' ? 12 : row.profile === 'pure' ? 10 : 8;
      const requiredMinSamplePoints = safeNumber(
        row.gatePayload?.profile_requirements?.empirical_min_sample_points,
        safeNumber(
          row.gatePayload?.profile_requirements?.proof_tracks?.empirical_min_sample_points,
          safeNumber(
          row.gatePayload?.effective_policy?.empirical_min_sample_points,
          safeNumber(
            row.gatePayload?.effective_policy?.proof_tracks?.empirical_min_sample_points,
            safeNumber(row.gatePayload?.empirical_min_sample_points, policyFallbackMin),
          ),
          ),
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
      const profileMinimumConfigured = requiredMinSamplePoints > 0;
      const samplePointsOk = row.empiricalSamplePoints >= requiredMinSamplePoints;
      const requiredSourcesOk = missingRequiredSourceIds.length === 0;
      const requiredMetricsOk = missingRequiredMetricKeys.length === 0;
      const requiredPositiveMetricsOk = nonPositiveRequiredMetricKeys.length === 0;
      const empiricalReleaseGateChecksFailed = Array.isArray(row.gatePayload?.checks)
        ? row.gatePayload.checks
            .filter((check: any) => {
              const id = cleanText(String(check?.id || ''), 120);
              const pass = check?.ok === true || check?.pass === true;
              return id.startsWith('proof_track_empirical_') && !pass;
            })
            .map((check: any) => cleanText(String(check?.id || ''), 120))
            .filter(Boolean)
        : [];
      const releaseGateNonEmpiricalChecksFailed = Array.isArray(row.gatePayload?.checks)
        ? row.gatePayload.checks
            .filter((check: any) => {
              const id = cleanText(String(check?.id || ''), 120);
              const pass = check?.ok === true || check?.pass === true;
              return id.length > 0 && !id.startsWith('proof_track_empirical_') && !pass;
            })
            .map((check: any) => cleanText(String(check?.id || ''), 120))
            .filter(Boolean)
        : [];
      const hasReleaseGateChecks = Array.isArray(row.gatePayload?.checks);
      const empiricalReleaseGateExecutionOk =
        hasReleaseGateChecks &&
        empiricalReleaseGateChecksFailed.length === 0;
      const releaseGateReasons: string[] = [];
      if (row.empiricalSamplePoints <= 0) {
        releaseGateReasons.push('empirical_sample_points_missing');
      }
      if (!profileMinimumConfigured) {
        releaseGateReasons.push('empirical_profile_minimum_missing');
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
        releaseGateReasons.push(
          hasReleaseGateChecks
            ? 'empirical_release_gate_checks_failed'
            : 'empirical_release_gate_payload_missing',
        );
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
        empirical_profile_minimum_configured: profileMinimumConfigured,
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
        release_gate_non_empirical_checks_failed: releaseGateNonEmpiricalChecksFailed,
        empirical_release_gate_reasons: releaseGateReasons,
        empirical_release_gate_pass: releaseGateReasons.length === 0,
        empirical_source_rows: sourceRows,
        empirical_required_source_rows: requiredSourceRows,
        empirical_required_metric_rows: requiredMetricRows,
        source_artifact: row.harnessOut,
        release_gate_artifact: row.gateOut,
      };
    });

  const empiricalMinimumContractRows = empiricalProfileCoverageRows.map((row) => ({
    profile: cleanText(row.profile || '', 40),
    empirical_min_sample_points_required: safeNumber(
      row.empirical_min_sample_points_required,
      0,
    ),
    empirical_profile_minimum_configured:
      row.empirical_profile_minimum_configured === true,
    empirical_sample_points: safeNumber(row.empirical_sample_points, 0),
    source_artifact: cleanText(row.source_artifact || '', 260),
    release_gate_artifact: cleanText(row.release_gate_artifact || '', 260),
  }));
  const minimumFloors = empiricalMinimumContractRows.map((row) =>
    safeNumber(row.empirical_min_sample_points_required, 0),
  );
  const missingMinimumProfileIds = empiricalMinimumContractRows
    .filter((row) => row.empirical_profile_minimum_configured !== true)
    .map((row) => cleanText(row.profile || '', 40))
    .filter(Boolean);
  const empiricalMinimumContract = {
    ok: missingMinimumProfileIds.length === 0,
    type: 'runtime_proof_empirical_minimum_contract',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    proof_track: args.proofTrack,
    summary: {
      profiles_total: empiricalMinimumContractRows.length,
      configured_profiles:
        empiricalMinimumContractRows.length - missingMinimumProfileIds.length,
      missing_profiles: missingMinimumProfileIds.length,
      missing_profile_ids: missingMinimumProfileIds,
      min_floor:
        minimumFloors.length > 0 ? Math.min(...minimumFloors) : 0,
      max_floor:
        minimumFloors.length > 0 ? Math.max(...minimumFloors) : 0,
    },
    profiles: empiricalMinimumContractRows,
  };
  writeJsonArtifact(empiricalMinimumContractOut, empiricalMinimumContract);
  ensureParentDir(empiricalMinimumContractMarkdownOut);
  fs.writeFileSync(
    empiricalMinimumContractMarkdownOut,
    renderEmpiricalMinimumContractMarkdown(empiricalMinimumContract),
    'utf8',
  );

  const empiricalProfileCoverage = {
    ok: empiricalProfileCoverageRows.every(
      (row) =>
        row.empirical_sample_points > 0 &&
        row.empirical_profile_minimum_configured &&
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
        row.empirical_profile_minimum_configured &&
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
      empirical_profile_minimum_configured:
        row.empirical_profile_minimum_configured === true,
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
      row.gatewayChaosOut,
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
      empiricalMinimumContractOut,
      empiricalMinimumContractMarkdownOut,
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

  const profileRunRows = profileRuns.map((row) => ({
    profile: row.profile,
    ok: row.ok,
    harness_exit: row.harnessExit,
    release_gate_exit: row.gateExit,
    gateway_runtime_chaos_exit: row.gatewayChaosExit,
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
      row.gatewayChaosOut,
    ],
  }));
  const artifactPaths = profileRunRows
    .flatMap((row) => row.artifact_paths)
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
      empiricalMinimumContractOut,
      empiricalMinimumContractMarkdownOut,
      args.empiricalHistoryPath,
      proofChecksumsOut,
    ]);
  const evidence = {
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
    empirical_minimum_contract: empiricalMinimumContractOut,
    empirical_minimum_contract_markdown: empiricalMinimumContractMarkdownOut,
    empirical_history: args.empiricalHistoryPath,
    proof_checksums: proofChecksumsOut,
  };
  const expectedProfilesForSelection =
    args.profile === 'all' ? ['rich', 'pure', 'tiny-max'] : [args.profile];
  const observedProfiles = profileRunRows.map((row) => cleanText(row.profile || '', 40));
  const observedProfileSet = new Set(observedProfiles);
  const observedProfilesUnique = observedProfiles.length === observedProfileSet.size;
  const expectedProfilesCovered = expectedProfilesForSelection.every((profile) =>
    observedProfileSet.has(cleanText(profile, 40)),
  );
  const noUnexpectedProfiles = Array.from(observedProfileSet).every((profile) =>
    expectedProfilesForSelection.includes(profile as any),
  );
  const profileArtifactPaths = profileRunRows.flatMap((row) => row.artifact_paths || []);
  const profileArtifactPathsUnique = profileArtifactPaths.length === new Set(profileArtifactPaths).size;
  const profileArtifactPathsTokenValid = profileArtifactPaths.every((artifactPath) =>
    isCanonicalArtifactToken(artifactPath),
  );
  const artifactPathsUnique = artifactPaths.length === new Set(artifactPaths).size;
  const artifactPathsTokenValid = artifactPaths.every((artifactPath) =>
    isCanonicalArtifactToken(artifactPath),
  );
  const checksumPathSet = new Set(
    checksumRows.map((row) => cleanText(row.path || '', 500)).filter(Boolean),
  );
  const checksumPathsUnique = checksumPathSet.size === checksumRows.length;
  const checksumPathsTokenValid = checksumRows.every((row) =>
    isCanonicalArtifactToken(row.path),
  );
  const checksumSha256Valid = checksumRows.every(
    (row) => row.exists === true && /^[a-f0-9]{64}$/i.test(cleanText(row.sha256 || '', 80)),
  );
  const artifactPathsSubsetChecksum = artifactPaths.every((artifactPath) =>
    checksumPathSet.has(cleanText(artifactPath || '', 500)),
  );
  const profileArtifactPathsSubsetTop = profileArtifactPaths.every((artifactPath) =>
    artifactPaths.includes(artifactPath),
  );
  const evidencePaths = Object.values(evidence).map((value) => cleanText(value || '', 500));
  const evidencePathsNonEmpty = evidencePaths.every((artifactPath) => artifactPath.length > 0);
  const evidencePathsUnique = evidencePaths.length === new Set(evidencePaths).size;
  const evidencePathsSubsetArtifactPaths = evidencePaths.every((artifactPath) =>
    artifactPaths.includes(artifactPath),
  );
  const summaryProfileCount = profileRunRows.length;
  const summaryProfilesPassed = profileRunRows.filter((row) => row.ok).length;
  const summaryBoundednessInspectExitMax =
    profileRuns.length > 0
      ? Math.max(...profileRuns.map((row) => row.boundednessInspectExit))
      : 0;
  const baseFailures = [
    ...profileRuns.flatMap((row) => [
      ...(row.boundednessInspectExit === 0
        ? []
        : [{
            id: 'runtime_boundedness_inspect_failed',
            detail: `profile=${row.profile};exit_code=${row.boundednessInspectExit}`,
          }]),
      ...(row.boundednessInspectPayload?.ok === true
        ? []
        : [{
            id: 'runtime_boundedness_inspect_report_not_ok',
            detail: `profile=${row.profile};artifact=${row.boundednessInspectOut}`,
          }]),
      ...(row.harnessExit === 0
        ? []
        : [{ id: 'runtime_proof_harness_failed', detail: `profile=${row.profile};exit_code=${row.harnessExit}` }]),
      ...(row.gateExit === 0
        ? []
        : args.proofTrack === 'empirical'
          ? []
          : [{ id: 'runtime_proof_release_gate_failed', detail: `profile=${row.profile};exit_code=${row.gateExit}` }]),
      ...(row.gatewayChaosExit === 0
        ? []
        : [{ id: 'gateway_runtime_chaos_gate_failed', detail: `profile=${row.profile};exit_code=${row.gatewayChaosExit}` }]),
      ...(row.empiricalGateOk
        ? []
        : [{ id: 'runtime_proof_empirical_sample_points_missing', detail: `profile=${row.profile};sample_points=${row.empiricalSamplePoints}` }]),
    ]),
    ...(boundednessEvidence.ok ? [] : [{ id: 'runtime_boundedness_72h_evidence_incomplete', detail: boundednessOut }]),
    ...(boundednessProfilesEvidence.ok
      ? []
      : [{ id: 'runtime_boundedness_profiles_evidence_incomplete', detail: boundednessProfilesOut }]),
    ...(multiDaySoakEvidence.ok ? [] : [{ id: 'runtime_multi_day_soak_evidence_incomplete', detail: multiDaySoakOut }]),
    ...(empiricalMinimumContract.ok
      ? []
      : [{ id: 'runtime_proof_empirical_minimum_contract_incomplete', detail: empiricalMinimumContractOut }]),
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
  ];
  const baseFailureIds = baseFailures.map((row) => cleanText(row.id || '', 160)).filter(Boolean);
  const baseFailureIdsUnique = baseFailureIds.length === new Set(baseFailureIds).size;
  const baseFailureIdsTokenValid = baseFailureIds.every((failureId) =>
    /^[a-z0-9:_-]+$/.test(failureId),
  );
  const baseFailureRowsObjectValid = baseFailures.every(
    (row) => row && typeof row === 'object' && !Array.isArray(row),
  );
  const contractFacts: Array<[string, boolean, string]> = [
    ['runtime_proof_verify_profile_selection_expected_set_contract_v2', expectedProfilesCovered && noUnexpectedProfiles, `expected=${expectedProfilesForSelection.join(',')};observed=${observedProfiles.join(',')}`],
    ['runtime_proof_verify_profile_ids_unique_contract_v2', observedProfilesUnique, `profiles=${observedProfiles.length};unique_profiles=${observedProfileSet.size}`],
    ['runtime_proof_verify_profile_artifact_paths_token_contract_v2', profileArtifactPathsTokenValid, `profile_artifact_paths=${profileArtifactPaths.length}`],
    ['runtime_proof_verify_profile_artifact_paths_unique_contract_v2', profileArtifactPathsUnique, `profile_artifact_paths=${profileArtifactPaths.length};unique_profile_artifact_paths=${new Set(profileArtifactPaths).size}`],
    ['runtime_proof_verify_profile_artifact_paths_subset_top_contract_v2', profileArtifactPathsSubsetTop, `profile_artifact_paths=${profileArtifactPaths.length};top_artifact_paths=${artifactPaths.length}`],
    ['runtime_proof_verify_summary_profile_count_contract_v2', summaryProfileCount === safeNumber(profileRuns.length, -1), `summary_profile_count=${summaryProfileCount};profile_runs=${profileRuns.length}`],
    ['runtime_proof_verify_summary_profiles_passed_contract_v2', summaryProfilesPassed === profileRunRows.filter((row) => row.ok).length, `summary_profiles_passed=${summaryProfilesPassed};computed_profiles_passed=${profileRunRows.filter((row) => row.ok).length}`],
    ['runtime_proof_verify_summary_boundedness_exit_max_contract_v2', summaryBoundednessInspectExitMax === (profileRuns.length > 0 ? Math.max(...profileRuns.map((row) => row.boundednessInspectExit)) : 0), `summary_boundedness_exit_max=${summaryBoundednessInspectExitMax}`],
    ['runtime_proof_verify_artifact_paths_token_contract_v2', artifactPathsTokenValid, `artifact_paths=${artifactPaths.length}`],
    ['runtime_proof_verify_artifact_paths_unique_contract_v2', artifactPathsUnique, `artifact_paths=${artifactPaths.length};unique_artifact_paths=${new Set(artifactPaths).size}`],
    ['runtime_proof_verify_artifact_paths_subset_checksum_contract_v2', artifactPathsSubsetChecksum, `artifact_paths=${artifactPaths.length};checksum_paths=${checksumPathSet.size}`],
    ['runtime_proof_verify_evidence_paths_nonempty_contract_v2', evidencePathsNonEmpty, `evidence_paths=${evidencePaths.length}`],
    ['runtime_proof_verify_evidence_paths_unique_contract_v2', evidencePathsUnique, `evidence_paths=${evidencePaths.length};unique_evidence_paths=${new Set(evidencePaths).size}`],
    ['runtime_proof_verify_evidence_paths_subset_artifact_paths_contract_v2', evidencePathsSubsetArtifactPaths, `evidence_paths=${evidencePaths.length};artifact_paths=${artifactPaths.length}`],
    ['runtime_proof_verify_checksum_paths_unique_contract_v2', checksumPathsUnique, `checksum_rows=${checksumRows.length};unique_checksum_paths=${checksumPathSet.size}`],
    ['runtime_proof_verify_checksum_paths_token_contract_v2', checksumPathsTokenValid, `checksum_rows=${checksumRows.length}`],
    ['runtime_proof_verify_checksum_sha256_contract_v2', checksumSha256Valid, `checksum_rows=${checksumRows.length}`],
    ['runtime_proof_verify_failure_ids_token_contract_v2', baseFailureIdsTokenValid, `failure_ids=${baseFailureIds.length}`],
    ['runtime_proof_verify_failure_ids_unique_contract_v2', baseFailureIdsUnique, `failure_ids=${baseFailureIds.length};unique_failure_ids=${new Set(baseFailureIds).size}`],
    ['runtime_proof_verify_failure_rows_object_contract_v2', baseFailureRowsObjectValid, `failure_rows=${baseFailures.length}`],
  ];
  let contractChecks = contractFacts.map(([id, ok, detail]) => ({ id, ok, detail }));
  const expectedProfileCountForSelection = args.profile === 'all' ? 3 : 1;
  const profileRunProfiles = profileRunRows
    .map((row) => cleanText(row.profile || '', 40))
    .filter(Boolean);
  const profileRunProfilesSet = new Set(profileRunProfiles);
  const profileRunProfilesSorted = Array.from(profileRunProfilesSet).sort().join(',');
  const observedProfilesSorted = Array.from(observedProfileSet).sort().join(',');
  const artifactPathsTrimmedStable = artifactPaths.every(
    (artifactPath) => artifactPath === cleanText(artifactPath || '', 500),
  );
  const artifactPathsNonEmpty = artifactPaths.every(
    (artifactPath) => cleanText(artifactPath || '', 500).length > 0,
  );
  const evidencePathsTokenValid = evidencePaths.every((artifactPath) =>
    isCanonicalArtifactToken(artifactPath),
  );
  const evidencePathsTrimmedStable = evidencePaths.every(
    (artifactPath) => artifactPath === cleanText(artifactPath || '', 500),
  );
  const checksumRowsObjectValid = checksumRows.every(
    (row) => row && typeof row === 'object' && !Array.isArray(row),
  );
  const checksumRowsExistsBoolean = checksumRows.every(
    (row) => typeof row.exists === 'boolean',
  );
  const checksumRowsShaPresentWhenExists = checksumRows.every(
    (row) =>
      row.exists === true
        ? /^[a-f0-9]{64}$/.test(cleanText(row.sha256 || '', 80))
        : true,
  );
  const checksumRowsShaEmptyWhenMissing = checksumRows.every(
    (row) =>
      row.exists === true ? true : cleanText(row.sha256 || '', 80).length === 0,
  );
  const profileRunRowsObjectValid = profileRunRows.every(
    (row) => row && typeof row === 'object' && !Array.isArray(row),
  );
  const profileRunRowsOkBoolean = profileRunRows.every(
    (row) => typeof row.ok === 'boolean',
  );
  const profileRunRowsProfileTokenValid = profileRunRows.every((row) =>
    /^[a-z0-9-]+$/.test(cleanText(row.profile || '', 40)),
  );
  const baseFailureRowsFieldPresence = baseFailures.every(
    (row) =>
      cleanText(row.id || '', 160).length > 0 &&
      cleanText(row.detail || '', 500).length > 0,
  );
  const baseFailureDetailsNonEmpty = baseFailures.every(
    (row) => cleanText(row.detail || '', 500).length > 0,
  );
  contractChecks = contractChecks.concat([
    {
      id: 'runtime_proof_verify_summary_profile_count_non_negative_contract_v3',
      ok: Number.isInteger(summaryProfileCount) && summaryProfileCount >= 0,
      detail: `summary_profile_count=${summaryProfileCount}`,
    },
    {
      id: 'runtime_proof_verify_summary_profiles_passed_non_negative_contract_v3',
      ok: Number.isInteger(summaryProfilesPassed) && summaryProfilesPassed >= 0,
      detail: `summary_profiles_passed=${summaryProfilesPassed}`,
    },
    {
      id: 'runtime_proof_verify_summary_profiles_passed_within_profile_count_contract_v3',
      ok: summaryProfilesPassed <= summaryProfileCount,
      detail: `summary_profiles_passed=${summaryProfilesPassed};summary_profile_count=${summaryProfileCount}`,
    },
    {
      id: 'runtime_proof_verify_summary_profile_count_expected_selector_contract_v3',
      ok: summaryProfileCount === expectedProfileCountForSelection,
      detail: `selector=${args.profile};expected=${expectedProfileCountForSelection};observed=${summaryProfileCount}`,
    },
    {
      id: 'runtime_proof_verify_profile_run_rows_object_contract_v3',
      ok: profileRunRowsObjectValid,
      detail: `profile_run_rows=${profileRunRows.length}`,
    },
    {
      id: 'runtime_proof_verify_profile_run_rows_ok_boolean_contract_v3',
      ok: profileRunRowsOkBoolean,
      detail: `profile_run_rows=${profileRunRows.length}`,
    },
    {
      id: 'runtime_proof_verify_profile_run_rows_profile_token_contract_v3',
      ok: profileRunRowsProfileTokenValid,
      detail: `profiles=${profileRunProfiles.join(',')}`,
    },
    {
      id: 'runtime_proof_verify_profile_run_rows_profile_set_alignment_contract_v3',
      ok: profileRunProfilesSorted === observedProfilesSorted,
      detail: `profile_runs=${profileRunProfilesSorted};observed=${observedProfilesSorted}`,
    },
    {
      id: 'runtime_proof_verify_artifact_paths_nonempty_contract_v3',
      ok: artifactPathsNonEmpty,
      detail: `artifact_paths=${artifactPaths.length}`,
    },
    {
      id: 'runtime_proof_verify_artifact_paths_trimmed_stable_contract_v3',
      ok: artifactPathsTrimmedStable,
      detail: `artifact_paths=${artifactPaths.length}`,
    },
    {
      id: 'runtime_proof_verify_evidence_paths_token_contract_v3',
      ok: evidencePathsTokenValid,
      detail: `evidence_paths=${evidencePaths.length}`,
    },
    {
      id: 'runtime_proof_verify_evidence_paths_trimmed_stable_contract_v3',
      ok: evidencePathsTrimmedStable,
      detail: `evidence_paths=${evidencePaths.length}`,
    },
    {
      id: 'runtime_proof_verify_checksum_rows_object_contract_v3',
      ok: checksumRowsObjectValid,
      detail: `checksum_rows=${checksumRows.length}`,
    },
    {
      id: 'runtime_proof_verify_checksum_rows_exists_boolean_contract_v3',
      ok: checksumRowsExistsBoolean,
      detail: `checksum_rows=${checksumRows.length}`,
    },
    {
      id: 'runtime_proof_verify_checksum_rows_sha_present_when_exists_contract_v3',
      ok: checksumRowsShaPresentWhenExists,
      detail: `checksum_rows=${checksumRows.length}`,
    },
    {
      id: 'runtime_proof_verify_checksum_rows_sha_empty_when_missing_contract_v3',
      ok: checksumRowsShaEmptyWhenMissing,
      detail: `checksum_rows=${checksumRows.length}`,
    },
    {
      id: 'runtime_proof_verify_base_failure_rows_field_presence_contract_v3',
      ok: baseFailureRowsFieldPresence,
      detail: `base_failure_rows=${baseFailures.length}`,
    },
    {
      id: 'runtime_proof_verify_base_failure_details_nonempty_contract_v3',
      ok: baseFailureDetailsNonEmpty,
      detail: `base_failure_rows=${baseFailures.length}`,
    },
  ]);
  const contractCheckIdsForUniqueness = contractChecks
    .map((row) => cleanText(row.id || '', 220))
    .filter(Boolean);
  const contractCheckIdsUnique =
    contractCheckIdsForUniqueness.length ===
    new Set(contractCheckIdsForUniqueness).size;
  const contractCheckIdsTokenValid = contractCheckIdsForUniqueness.every((id) =>
    /^[a-z0-9:_-]+$/.test(id),
  );
  contractChecks = contractChecks.concat([
    {
      id: 'runtime_proof_verify_contract_check_ids_unique_contract_v3',
      ok: contractCheckIdsUnique,
      detail:
        `checks=${contractCheckIdsForUniqueness.length};` +
        `unique_checks=${new Set(contractCheckIdsForUniqueness).size}`,
    },
    {
      id: 'runtime_proof_verify_contract_check_ids_token_contract_v3',
      ok: contractCheckIdsTokenValid,
      detail: `checks=${contractCheckIdsForUniqueness.length}`,
    },
  ]);
  const empiricalProfileCoverageProfileIds = empiricalProfileCoverageRows
    .map((row) => cleanText(row.profile || '', 40))
    .filter(Boolean);
  const empiricalProfileGateProfiles = Array.isArray(empiricalProfileGate?.profiles)
    ? empiricalProfileGate.profiles
    : [];
  const empiricalProfileGateProfileIds = empiricalProfileGateProfiles
    .map((row: any) => cleanText(row?.profile || '', 40))
    .filter(Boolean);
  const empiricalProfileGateSummaryTotal = safeNumber(
    empiricalProfileGate?.summary?.profiles_total,
    -1,
  );
  const empiricalProfileGateSummaryFailed = safeNumber(
    empiricalProfileGate?.summary?.profiles_failed,
    -1,
  );
  const empiricalProfileGateSummaryPassed = safeNumber(
    empiricalProfileGate?.summary?.profiles_passed,
    -1,
  );
  const empiricalProfileGateFailingProfileIds = empiricalProfileGateProfiles
    .filter((row: any) => row?.empirical_release_gate_pass !== true)
    .map((row: any) => cleanText(row?.profile || '', 40))
    .filter(Boolean);
  const empiricalProfileGateFailingFromSummary = toStringArray(
    empiricalProfileGate?.summary?.failing_profiles,
    40,
  );
  const empiricalProfileReadinessProfiles = Array.isArray(empiricalProfileReadiness?.profiles)
    ? empiricalProfileReadiness.profiles
    : [];
  const empiricalProfileReadinessProfileIds = empiricalProfileReadinessProfiles
    .map((row: any) => cleanText(row?.profile || '', 40))
    .filter(Boolean);
  const empiricalProfileReadinessSummaryTotal = safeNumber(
    empiricalProfileReadiness?.summary?.profiles_total,
    -1,
  );
  const empiricalProfileReadinessSummaryReleaseReady = safeNumber(
    empiricalProfileReadiness?.summary?.release_ready_profiles,
    -1,
  );
  const empiricalProfileReadinessSummaryDegraded = safeNumber(
    empiricalProfileReadiness?.summary?.degraded_profiles,
    -1,
  );
  const empiricalProfileReadinessSummaryBlocked = safeNumber(
    empiricalProfileReadiness?.summary?.blocked_profiles,
    -1,
  );
  const empiricalProfileReadinessSummaryAvg = safeNumber(
    empiricalProfileReadiness?.summary?.readiness_score_avg,
    0,
  );
  const empiricalProfileReadinessDerivedReleaseReady = empiricalProfileReadinessProfiles.filter(
    (row: any) => cleanText(row?.readiness_class || '', 40) === 'release_ready',
  ).length;
  const empiricalProfileReadinessDerivedDegraded = empiricalProfileReadinessProfiles.filter(
    (row: any) => cleanText(row?.readiness_class || '', 40) === 'degraded',
  ).length;
  const empiricalProfileReadinessDerivedBlocked = empiricalProfileReadinessProfiles.filter(
    (row: any) => cleanText(row?.readiness_class || '', 40) === 'blocked',
  ).length;
  const empiricalProfileReadinessScoreTotal = empiricalProfileReadinessProfiles.reduce(
    (acc, row: any) => acc + safeNumber(row?.readiness_score, 0),
    0,
  );
  const empiricalProfileReadinessDerivedAvg =
    empiricalProfileReadinessProfiles.length > 0
      ? Math.round(
          (empiricalProfileReadinessScoreTotal /
            empiricalProfileReadinessProfiles.length) *
            100,
        ) / 100
      : 0;
  const proofChecksumRows = Array.isArray(proofChecksums?.checksums)
    ? proofChecksums.checksums
    : [];
  const proofChecksumPathSet = new Set(
    proofChecksumRows
      .map((row: any) => cleanText(row?.path || '', 500))
      .filter(Boolean),
  );
  const checksumRowsPathSet = new Set(
    checksumRows
      .map((row) => cleanText(row.path || '', 500))
      .filter(Boolean),
  );
  const proofChecksumPathSetMatches =
    proofChecksumPathSet.size === checksumRowsPathSet.size &&
    Array.from(checksumRowsPathSet).every((pathToken) =>
      proofChecksumPathSet.has(pathToken),
    );
  const profileRunArtifactPathsNonEmptyPerRun = profileRunRows.every((row) => {
    const paths = Array.isArray(row.artifact_paths) ? row.artifact_paths : [];
    return (
      paths.length > 0 &&
      paths.every((artifactPath: any) => cleanText(artifactPath || '', 500).length > 0)
    );
  });
  const profileRunArtifactPathsUniquePerRun = profileRunRows.every((row) => {
    const paths = Array.isArray(row.artifact_paths)
      ? row.artifact_paths.map((artifactPath: any) => cleanText(artifactPath || '', 500))
      : [];
    return paths.length === new Set(paths).size;
  });
  const profileRunArtifactPathsTokenPerRun = profileRunRows.every((row) => {
    const paths = Array.isArray(row.artifact_paths) ? row.artifact_paths : [];
    return paths.every((artifactPath: any) => isCanonicalArtifactToken(artifactPath));
  });
  const releaseGateQualityPaths = RELEASE_GATE_QUALITY_PATHS.map((artifactPath) =>
    cleanText(artifactPath || '', 500),
  );
  const releaseGateQualityPathsTokenValid = releaseGateQualityPaths.every((artifactPath) =>
    isCanonicalArtifactToken(artifactPath),
  );
  const releaseGateQualityPathsUnique =
    releaseGateQualityPaths.length === new Set(releaseGateQualityPaths).size;
  const empiricalProfileIdTokenRegex = /^[a-z0-9-]+$/;
  const empiricalCoverageProfileIdsTokenValid = empiricalProfileCoverageProfileIds.every(
    (profileId) => empiricalProfileIdTokenRegex.test(cleanText(profileId || '', 40)),
  );
  const empiricalCoverageProfileIdsUnique =
    empiricalProfileCoverageProfileIds.length ===
    new Set(empiricalProfileCoverageProfileIds).size;
  const empiricalGateProfileIdsTokenValid = empiricalProfileGateProfileIds.every((profileId) =>
    empiricalProfileIdTokenRegex.test(cleanText(profileId || '', 40)),
  );
  const empiricalGateProfileIdsUnique =
    empiricalProfileGateProfileIds.length ===
    new Set(empiricalProfileGateProfileIds).size;
  const empiricalReadinessProfileIdsTokenValid = empiricalProfileReadinessProfileIds.every(
    (profileId) => empiricalProfileIdTokenRegex.test(cleanText(profileId || '', 40)),
  );
  const empiricalReadinessProfileIdsUnique =
    empiricalProfileReadinessProfileIds.length ===
    new Set(empiricalProfileReadinessProfileIds).size;
  const empiricalCoverageProfileSet = new Set(empiricalProfileCoverageProfileIds);
  const empiricalGateProfileSet = new Set(empiricalProfileGateProfileIds);
  const empiricalReadinessProfileSet = new Set(empiricalProfileReadinessProfileIds);
  const empiricalCoverageGateSetAligned =
    empiricalCoverageProfileSet.size === empiricalGateProfileSet.size &&
    Array.from(empiricalCoverageProfileSet).every((profileId) =>
      empiricalGateProfileSet.has(profileId),
    );
  const empiricalCoverageReadinessSetAligned =
    empiricalCoverageProfileSet.size === empiricalReadinessProfileSet.size &&
    Array.from(empiricalCoverageProfileSet).every((profileId) =>
      empiricalReadinessProfileSet.has(profileId),
    );
  const empiricalGateReadinessSetAligned =
    empiricalGateProfileSet.size === empiricalReadinessProfileSet.size &&
    Array.from(empiricalGateProfileSet).every((profileId) =>
      empiricalReadinessProfileSet.has(profileId),
    );
  const empiricalGateSummaryReasonBuckets = empiricalProfileGate?.summary?.reason_buckets;
  const empiricalGateSummaryReasonBucketRows = empiricalGateSummaryReasonBuckets &&
    typeof empiricalGateSummaryReasonBuckets === 'object' &&
    !Array.isArray(empiricalGateSummaryReasonBuckets)
    ? Object.entries(empiricalGateSummaryReasonBuckets).map(([reason, count]) => ({
        reason: cleanText(reason || '', 120),
        count: safeNumber(count, 0),
      }))
    : [];
  const empiricalGateSummaryReasonBucketsDerived = empiricalProfileGateProfiles.reduce(
    (acc, row: any) => {
      const reasons = toStringArray(row?.empirical_release_gate_reasons, 160);
      for (const reason of reasons) {
        const key = cleanText(reason, 120);
        if (!key) continue;
        acc[key] = safeNumber(acc[key], 0) + 1;
      }
      return acc;
    },
    {} as Record<string, number>,
  );
  const empiricalGateReasonBucketParity =
    empiricalGateSummaryReasonBucketRows.length ===
      Object.keys(empiricalGateSummaryReasonBucketsDerived).length &&
    empiricalGateSummaryReasonBucketRows.every(
      (row) => row.reason.length > 0 && row.count === safeNumber(empiricalGateSummaryReasonBucketsDerived[row.reason], 0),
    );
  const empiricalFailingProfilesBaseSet = new Set(
    failingProfiles.map((profileId) => cleanText(profileId || '', 40)).filter(Boolean),
  );
  const empiricalFailingProfilesGateSet = new Set(empiricalProfileGateFailingProfileIds);
  const empiricalFailingProfilesAligned =
    empiricalFailingProfilesBaseSet.size === empiricalFailingProfilesGateSet.size &&
    Array.from(empiricalFailingProfilesBaseSet).every((profileId) =>
      empiricalFailingProfilesGateSet.has(profileId),
    );
  const empiricalReadinessClassTokenValid = empiricalProfileReadinessProfiles.every((row: any) =>
    ['release_ready', 'degraded', 'blocked'].includes(
      cleanText(row?.readiness_class || '', 40),
    ),
  );
  const empiricalReadinessReasonsTokenValid = empiricalProfileReadinessProfiles.every((row: any) =>
    toStringArray(row?.reasons, 160).every((reason) =>
      /^[a-z0-9_:-]+$/.test(cleanText(reason || '', 160)),
    ),
  );
  const empiricalReadinessReasonsUnique = empiricalProfileReadinessProfiles.every((row: any) => {
    const reasons = toStringArray(row?.reasons, 160);
    return reasons.length === new Set(reasons).size;
  });
  const empiricalReadinessSevereReasonsSubset = empiricalProfileReadinessProfiles.every((row: any) => {
    const reasons = toStringArray(row?.reasons, 160);
    const severeReasons = toStringArray(row?.severe_reasons, 160);
    return severeReasons.every((reason) => reasons.includes(reason));
  });
  const empiricalReadinessSevereReasonsTokenValid = empiricalProfileReadinessProfiles.every(
    (row: any) =>
      toStringArray(row?.severe_reasons, 160).every((reason) =>
        /^[a-z0-9_:-]+$/.test(cleanText(reason || '', 160)),
      ),
  );
  const empiricalReadinessSevereReasonsUnique = empiricalProfileReadinessProfiles.every(
    (row: any) => {
      const severeReasons = toStringArray(row?.severe_reasons, 160);
      return severeReasons.length === new Set(severeReasons).size;
    },
  );
  const empiricalGateChecksFailedTokenValid = empiricalProfileGateProfiles.every((row: any) =>
    toStringArray(row?.empirical_release_gate_checks_failed, 160).every((checkId) =>
      /^[a-z0-9_:-]+$/.test(cleanText(checkId || '', 160)),
    ),
  );
  const empiricalGateChecksFailedUnique = empiricalProfileGateProfiles.every((row: any) => {
    const checksFailed = toStringArray(row?.empirical_release_gate_checks_failed, 160);
    return checksFailed.length === new Set(checksFailed).size;
  });
  const proofChecksumsOkDerived = proofChecksumRows.every(
    (row: any) =>
      row?.exists === true &&
      /^[a-f0-9]{64}$/i.test(cleanText(row?.sha256 || '', 80)),
  );
  const empiricalCoverageSamplePointsNonNegative = empiricalProfileCoverageRows.every(
    (row) => safeNumber(row.empirical_sample_points, -1) >= 0,
  );
  const empiricalCoverageMinRequiredNonNegative = empiricalProfileCoverageRows.every(
    (row) => safeNumber(row.empirical_min_sample_points_required, -1) >= 0,
  );
  const empiricalCoverageSamplePointsOkImpliesAtLeastMin = empiricalProfileCoverageRows.every(
    (row) =>
      row.empirical_sample_points_ok !== true ||
      safeNumber(row.empirical_sample_points, -1) >=
        safeNumber(row.empirical_min_sample_points_required, -1),
  );
  const empiricalCoverageProvidedKeysCountMatches = empiricalProfileCoverageRows.every(
    (row) =>
      safeNumber(row.empirical_provided_keys_count, -1) ===
      toStringArray(row.empirical_provided_keys, 120).length,
  );
  const empiricalCoverageRequiredSourcesMissingUnique = empiricalProfileCoverageRows.every(
    (row) => {
      const missing = toStringArray(row.empirical_required_sources_missing, 120);
      return missing.length === new Set(missing).size;
    },
  );
  const empiricalCoverageRequiredMetricsMissingUnique = empiricalProfileCoverageRows.every(
    (row) => {
      const missing = toStringArray(row.empirical_required_metrics_missing, 120);
      return missing.length === new Set(missing).size;
    },
  );
  const empiricalCoverageRequiredPositiveMetricsMissingUnique =
    empiricalProfileCoverageRows.every((row) => {
      const missing = toStringArray(
        row.empirical_required_positive_metrics_missing,
        120,
      );
      return missing.length === new Set(missing).size;
    });
  const empiricalRequiredSourceRowsTokenValid = empiricalProfileCoverageRows.every((row) => {
    const requiredRows = Array.isArray(row.empirical_required_source_rows)
      ? row.empirical_required_source_rows
      : [];
    return requiredRows.every((requiredRow: any) =>
      /^[a-z0-9_:-]+$/.test(cleanText(requiredRow?.id || '', 120)),
    );
  });
  const empiricalRequiredSourceRowsUnique = empiricalProfileCoverageRows.every((row) => {
    const requiredRows = Array.isArray(row.empirical_required_source_rows)
      ? row.empirical_required_source_rows
      : [];
    const ids = requiredRows
      .map((requiredRow: any) => cleanText(requiredRow?.id || '', 120))
      .filter(Boolean);
    return ids.length === new Set(ids).size;
  });
  const empiricalRequiredSourceRowsMissingConsistency = empiricalProfileCoverageRows.every(
    (row) => {
      const requiredRows = Array.isArray(row.empirical_required_source_rows)
        ? row.empirical_required_source_rows
        : [];
      return requiredRows.every((requiredRow: any) =>
        requiredRow?.required_missing === true
          ? requiredRow?.required_satisfied !== true
          : true,
      );
    },
  );
  const empiricalRequiredSourceRowsSatisfiedConsistency =
    empiricalProfileCoverageRows.every((row) => {
      const requiredRows = Array.isArray(row.empirical_required_source_rows)
        ? row.empirical_required_source_rows
        : [];
      return requiredRows.every((requiredRow: any) => {
        const samplePoints = safeNumber(requiredRow?.sample_points, 0);
        if (requiredRow?.required_satisfied !== true) return true;
        return (
          requiredRow?.required_missing !== true &&
          (requiredRow?.loaded === true || samplePoints > 0)
        );
      });
    });
  const empiricalRequiredMetricRowsTokenValid = empiricalProfileCoverageRows.every((row) => {
    const requiredRows = Array.isArray(row.empirical_required_metric_rows)
      ? row.empirical_required_metric_rows
      : [];
    return requiredRows.every((requiredRow: any) =>
      /^[a-z0-9_:-]+$/.test(cleanText(requiredRow?.key || '', 120)),
    );
  });
  const empiricalRequiredMetricRowsUnique = empiricalProfileCoverageRows.every((row) => {
    const requiredRows = Array.isArray(row.empirical_required_metric_rows)
      ? row.empirical_required_metric_rows
      : [];
    const keys = requiredRows
      .map((requiredRow: any) => cleanText(requiredRow?.key || '', 120))
      .filter(Boolean);
    return keys.length === new Set(keys).size;
  });
  const empiricalRequiredMetricRowsSatisfiedConsistency =
    empiricalProfileCoverageRows.every((row) => {
      const requiredRows = Array.isArray(row.empirical_required_metric_rows)
        ? row.empirical_required_metric_rows
        : [];
      return requiredRows.every((requiredRow: any) => {
        const requiredSatisfied = requiredRow?.required_satisfied === true;
        const derivedSatisfied =
          requiredRow?.required_missing !== true &&
          requiredRow?.required_non_positive !== true;
        return requiredSatisfied === derivedSatisfied;
      });
    });
  const empiricalRequiredMetricRowsValueNumeric = empiricalProfileCoverageRows.every((row) => {
    const requiredRows = Array.isArray(row.empirical_required_metric_rows)
      ? row.empirical_required_metric_rows
      : [];
    return requiredRows.every((requiredRow: any) =>
      Number.isFinite(Number(requiredRow?.value)),
    );
  });
  const empiricalSourceRowsTokenValid = empiricalProfileCoverageRows.every((row) => {
    const sourceRows = Array.isArray(row.empirical_source_rows)
      ? row.empirical_source_rows
      : [];
    return sourceRows.every((sourceRow: any) =>
      /^[a-z0-9_:-]+$/.test(cleanText(sourceRow?.id || '', 120)),
    );
  });
  const empiricalSourceRowsUnique = empiricalProfileCoverageRows.every((row) => {
    const sourceRows = Array.isArray(row.empirical_source_rows)
      ? row.empirical_source_rows
      : [];
    const ids = sourceRows
      .map((sourceRow: any) => cleanText(sourceRow?.id || '', 120))
      .filter(Boolean);
    return ids.length === new Set(ids).size;
  });
  const empiricalSourceRowsSamplePointsNonNegative = empiricalProfileCoverageRows.every(
    (row) => {
      const sourceRows = Array.isArray(row.empirical_source_rows)
        ? row.empirical_source_rows
        : [];
      return sourceRows.every((sourceRow: any) => safeNumber(sourceRow?.sample_points, -1) >= 0);
    },
  );
  const empiricalGateChecksFailedReasonParity = empiricalProfileGateProfiles.every(
    (row: any) => {
      const checksFailed = toStringArray(row?.empirical_release_gate_checks_failed, 160);
      const reasons = toStringArray(row?.empirical_release_gate_reasons, 160);
      const hasReason = reasons.includes('empirical_release_gate_checks_failed');
      return checksFailed.length > 0 ? hasReason : !hasReason;
    },
  );
  const empiricalGatePassReasonsConsistency = empiricalProfileGateProfiles.every(
    (row: any) => {
      const pass = row?.empirical_release_gate_pass === true;
      const reasons = toStringArray(row?.empirical_release_gate_reasons, 160);
      return pass ? reasons.length === 0 : reasons.length > 0;
    },
  );
  contractChecks = contractChecks.concat([
    {
      id: 'runtime_proof_verify_proof_track_token_contract_v4',
      ok: ['synthetic', 'empirical', 'dual'].includes(cleanText(args.proofTrack || '', 24)),
      detail: `proof_track=${cleanText(args.proofTrack || '', 24)}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_coverage_count_expected_selector_contract_v4',
      ok: empiricalProfileCoverageRows.length === expectedProfileCountForSelection,
      detail:
        `selector=${args.profile};expected=${expectedProfileCountForSelection};` +
        `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_count_expected_selector_contract_v4',
      ok: empiricalProfileGateProfiles.length === expectedProfileCountForSelection,
      detail:
        `selector=${args.profile};expected=${expectedProfileCountForSelection};` +
        `gate_rows=${empiricalProfileGateProfiles.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_readiness_count_expected_selector_contract_v4',
      ok: empiricalProfileReadinessProfiles.length === expectedProfileCountForSelection,
      detail:
        `selector=${args.profile};expected=${expectedProfileCountForSelection};` +
        `readiness_rows=${empiricalProfileReadinessProfiles.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_summary_total_contract_v4',
      ok: empiricalProfileGateSummaryTotal === empiricalProfileGateProfiles.length,
      detail:
        `summary_total=${empiricalProfileGateSummaryTotal};` +
        `rows=${empiricalProfileGateProfiles.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_summary_failed_contract_v4',
      ok: empiricalProfileGateSummaryFailed === empiricalProfileGateFailingProfileIds.length,
      detail:
        `summary_failed=${empiricalProfileGateSummaryFailed};` +
        `derived_failed=${empiricalProfileGateFailingProfileIds.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_summary_passed_contract_v4',
      ok:
        empiricalProfileGateSummaryPassed ===
        empiricalProfileGateProfiles.length - empiricalProfileGateFailingProfileIds.length,
      detail:
        `summary_passed=${empiricalProfileGateSummaryPassed};` +
        `derived_passed=${empiricalProfileGateProfiles.length - empiricalProfileGateFailingProfileIds.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_summary_failing_profiles_contract_v4',
      ok:
        empiricalProfileGateFailingFromSummary.length ===
          empiricalProfileGateFailingProfileIds.length &&
        empiricalProfileGateFailingFromSummary.every((profileId) =>
          empiricalProfileGateFailingProfileIds.includes(profileId),
        ) &&
        empiricalProfileGateFailingProfileIds.every((profileId) =>
          empiricalProfileGateFailingFromSummary.includes(profileId),
        ),
      detail:
        `summary=${empiricalProfileGateFailingFromSummary.join(',') || 'none'};` +
        `derived=${empiricalProfileGateFailingProfileIds.join(',') || 'none'}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_readiness_summary_total_contract_v4',
      ok:
        empiricalProfileReadinessSummaryTotal ===
        empiricalProfileReadinessProfiles.length,
      detail:
        `summary_total=${empiricalProfileReadinessSummaryTotal};` +
        `rows=${empiricalProfileReadinessProfiles.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_readiness_summary_release_ready_contract_v4',
      ok:
        empiricalProfileReadinessSummaryReleaseReady ===
        empiricalProfileReadinessDerivedReleaseReady,
      detail:
        `summary_release_ready=${empiricalProfileReadinessSummaryReleaseReady};` +
        `derived_release_ready=${empiricalProfileReadinessDerivedReleaseReady}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_readiness_summary_degraded_contract_v4',
      ok:
        empiricalProfileReadinessSummaryDegraded ===
        empiricalProfileReadinessDerivedDegraded,
      detail:
        `summary_degraded=${empiricalProfileReadinessSummaryDegraded};` +
        `derived_degraded=${empiricalProfileReadinessDerivedDegraded}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_readiness_summary_blocked_contract_v4',
      ok:
        empiricalProfileReadinessSummaryBlocked ===
        empiricalProfileReadinessDerivedBlocked,
      detail:
        `summary_blocked=${empiricalProfileReadinessSummaryBlocked};` +
        `derived_blocked=${empiricalProfileReadinessDerivedBlocked}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_readiness_summary_avg_contract_v4',
      ok: Math.abs(empiricalProfileReadinessSummaryAvg - empiricalProfileReadinessDerivedAvg) <= 0.0001,
      detail:
        `summary_avg=${empiricalProfileReadinessSummaryAvg};` +
        `derived_avg=${empiricalProfileReadinessDerivedAvg}`,
    },
    {
      id: 'runtime_proof_verify_proof_checksums_row_count_contract_v4',
      ok: proofChecksumRows.length === checksumRows.length,
      detail: `proof_checksum_rows=${proofChecksumRows.length};checksum_rows=${checksumRows.length}`,
    },
    {
      id: 'runtime_proof_verify_proof_checksums_path_set_contract_v4',
      ok: proofChecksumPathSetMatches,
      detail:
        `proof_checksum_paths=${proofChecksumPathSet.size};` +
        `checksum_paths=${checksumRowsPathSet.size}`,
    },
    {
      id: 'runtime_proof_verify_profile_run_artifact_paths_nonempty_contract_v4',
      ok: profileRunArtifactPathsNonEmptyPerRun,
      detail: `profile_runs=${profileRunRows.length}`,
    },
    {
      id: 'runtime_proof_verify_profile_run_artifact_paths_unique_per_run_contract_v4',
      ok: profileRunArtifactPathsUniquePerRun,
      detail: `profile_runs=${profileRunRows.length}`,
    },
    {
      id: 'runtime_proof_verify_profile_run_artifact_paths_token_per_run_contract_v4',
      ok: profileRunArtifactPathsTokenPerRun,
      detail: `profile_runs=${profileRunRows.length}`,
    },
    {
      id: 'runtime_proof_verify_release_gate_quality_paths_token_contract_v4',
      ok: releaseGateQualityPathsTokenValid,
      detail: releaseGateQualityPaths.join(','),
    },
    {
      id: 'runtime_proof_verify_release_gate_quality_paths_unique_contract_v4',
      ok: releaseGateQualityPathsUnique,
      detail:
        `quality_paths=${releaseGateQualityPaths.length};` +
        `unique_quality_paths=${new Set(releaseGateQualityPaths).size}`,
    },
    {
      id: 'runtime_proof_verify_empirical_coverage_profile_ids_token_contract_v5',
      ok: empiricalCoverageProfileIdsTokenValid,
      detail: empiricalProfileCoverageProfileIds.join(','),
    },
    {
      id: 'runtime_proof_verify_empirical_coverage_profile_ids_unique_contract_v5',
      ok: empiricalCoverageProfileIdsUnique,
      detail:
        `coverage_profiles=${empiricalProfileCoverageProfileIds.length};` +
        `unique_coverage_profiles=${new Set(empiricalProfileCoverageProfileIds).size}`,
    },
    {
      id: 'runtime_proof_verify_empirical_gate_profile_ids_token_contract_v5',
      ok: empiricalGateProfileIdsTokenValid,
      detail: empiricalProfileGateProfileIds.join(','),
    },
    {
      id: 'runtime_proof_verify_empirical_gate_profile_ids_unique_contract_v5',
      ok: empiricalGateProfileIdsUnique,
      detail:
        `gate_profiles=${empiricalProfileGateProfileIds.length};` +
        `unique_gate_profiles=${new Set(empiricalProfileGateProfileIds).size}`,
    },
    {
      id: 'runtime_proof_verify_empirical_readiness_profile_ids_token_contract_v5',
      ok: empiricalReadinessProfileIdsTokenValid,
      detail: empiricalProfileReadinessProfileIds.join(','),
    },
    {
      id: 'runtime_proof_verify_empirical_readiness_profile_ids_unique_contract_v5',
      ok: empiricalReadinessProfileIdsUnique,
      detail:
        `readiness_profiles=${empiricalProfileReadinessProfileIds.length};` +
        `unique_readiness_profiles=${new Set(empiricalProfileReadinessProfileIds).size}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_set_alignment_coverage_gate_contract_v5',
      ok: empiricalCoverageGateSetAligned,
      detail:
        `coverage=${Array.from(empiricalCoverageProfileSet).sort().join(',')};` +
        `gate=${Array.from(empiricalGateProfileSet).sort().join(',')}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_set_alignment_coverage_readiness_contract_v5',
      ok: empiricalCoverageReadinessSetAligned,
      detail:
        `coverage=${Array.from(empiricalCoverageProfileSet).sort().join(',')};` +
        `readiness=${Array.from(empiricalReadinessProfileSet).sort().join(',')}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_set_alignment_gate_readiness_contract_v5',
      ok: empiricalGateReadinessSetAligned,
      detail:
        `gate=${Array.from(empiricalGateProfileSet).sort().join(',')};` +
        `readiness=${Array.from(empiricalReadinessProfileSet).sort().join(',')}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_reason_bucket_parity_contract_v5',
      ok: empiricalGateReasonBucketParity,
      detail:
        `summary_rows=${empiricalGateSummaryReasonBucketRows.length};` +
        `derived_rows=${Object.keys(empiricalGateSummaryReasonBucketsDerived).length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_failing_profiles_alignment_contract_v5',
      ok: empiricalFailingProfilesAligned,
      detail:
        `base=${Array.from(empiricalFailingProfilesBaseSet).sort().join(',') || 'none'};` +
        `gate=${Array.from(empiricalFailingProfilesGateSet).sort().join(',') || 'none'}`,
    },
    {
      id: 'runtime_proof_verify_empirical_readiness_class_token_contract_v5',
      ok: empiricalReadinessClassTokenValid,
      detail: `readiness_profiles=${empiricalProfileReadinessProfiles.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_readiness_reasons_token_contract_v5',
      ok: empiricalReadinessReasonsTokenValid,
      detail: `readiness_profiles=${empiricalProfileReadinessProfiles.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_readiness_reasons_unique_contract_v5',
      ok: empiricalReadinessReasonsUnique,
      detail: `readiness_profiles=${empiricalProfileReadinessProfiles.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_readiness_severe_reasons_subset_contract_v5',
      ok: empiricalReadinessSevereReasonsSubset,
      detail: `readiness_profiles=${empiricalProfileReadinessProfiles.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_readiness_severe_reasons_token_contract_v5',
      ok: empiricalReadinessSevereReasonsTokenValid,
      detail: `readiness_profiles=${empiricalProfileReadinessProfiles.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_readiness_severe_reasons_unique_contract_v5',
      ok: empiricalReadinessSevereReasonsUnique,
      detail: `readiness_profiles=${empiricalProfileReadinessProfiles.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_gate_checks_failed_token_contract_v5',
      ok: empiricalGateChecksFailedTokenValid,
      detail: `gate_profiles=${empiricalProfileGateProfiles.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_gate_checks_failed_unique_contract_v5',
      ok: empiricalGateChecksFailedUnique,
      detail: `gate_profiles=${empiricalProfileGateProfiles.length}`,
    },
    {
      id: 'runtime_proof_verify_proof_checksums_ok_consistency_contract_v5',
      ok: Boolean(proofChecksums?.ok) === proofChecksumsOkDerived,
      detail:
        `reported_ok=${String(Boolean(proofChecksums?.ok))};` +
        `derived_ok=${String(proofChecksumsOkDerived)}`,
    },
    {
      id: 'runtime_proof_verify_empirical_coverage_sample_points_non_negative_contract_v6',
      ok: empiricalCoverageSamplePointsNonNegative,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_coverage_min_required_non_negative_contract_v6',
      ok: empiricalCoverageMinRequiredNonNegative,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_coverage_sample_points_ok_implies_min_contract_v6',
      ok: empiricalCoverageSamplePointsOkImpliesAtLeastMin,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_coverage_provided_keys_count_contract_v6',
      ok: empiricalCoverageProvidedKeysCountMatches,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_coverage_required_sources_missing_unique_contract_v6',
      ok: empiricalCoverageRequiredSourcesMissingUnique,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_coverage_required_metrics_missing_unique_contract_v6',
      ok: empiricalCoverageRequiredMetricsMissingUnique,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_coverage_required_positive_metrics_missing_unique_contract_v6',
      ok: empiricalCoverageRequiredPositiveMetricsMissingUnique,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_required_source_rows_token_contract_v6',
      ok: empiricalRequiredSourceRowsTokenValid,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_required_source_rows_unique_contract_v6',
      ok: empiricalRequiredSourceRowsUnique,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_required_source_rows_missing_consistency_contract_v6',
      ok: empiricalRequiredSourceRowsMissingConsistency,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_required_source_rows_satisfied_consistency_contract_v6',
      ok: empiricalRequiredSourceRowsSatisfiedConsistency,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_required_metric_rows_token_contract_v6',
      ok: empiricalRequiredMetricRowsTokenValid,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_required_metric_rows_unique_contract_v6',
      ok: empiricalRequiredMetricRowsUnique,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_required_metric_rows_satisfied_consistency_contract_v6',
      ok: empiricalRequiredMetricRowsSatisfiedConsistency,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_required_metric_rows_value_numeric_contract_v6',
      ok: empiricalRequiredMetricRowsValueNumeric,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_source_rows_token_contract_v6',
      ok: empiricalSourceRowsTokenValid,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_source_rows_unique_contract_v6',
      ok: empiricalSourceRowsUnique,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_source_rows_sample_points_non_negative_contract_v6',
      ok: empiricalSourceRowsSamplePointsNonNegative,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_gate_checks_failed_reason_parity_contract_v6',
      ok: empiricalGateChecksFailedReasonParity,
      detail: `gate_profiles=${empiricalProfileGateProfiles.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_gate_pass_reasons_consistency_contract_v6',
      ok: empiricalGatePassReasonsConsistency,
      detail: `gate_profiles=${empiricalProfileGateProfiles.length}`,
    },
  ]);
  const empiricalCoverageReleaseGateReasonsTokenValid = empiricalProfileCoverageRows.every((row) =>
    toStringArray(row.empirical_release_gate_reasons, 160).every((reason) =>
      /^[a-z0-9:_-]+$/.test(cleanText(reason, 120)),
    ),
  );
  const empiricalCoverageReleaseGateReasonsUnique = empiricalProfileCoverageRows.every((row) => {
    const reasons = toStringArray(row.empirical_release_gate_reasons, 160);
    return reasons.length === new Set(reasons).size;
  });
  const empiricalCoverageChecksFailedReasonParity = empiricalProfileCoverageRows.every((row) => {
    const reasons = toStringArray(row.empirical_release_gate_reasons, 160);
    const checksFailed = toStringArray(row.empirical_release_gate_checks_failed, 160);
    return checksFailed.length === 0 || reasons.includes('empirical_release_gate_checks_failed');
  });
  const empiricalCoverageGatePassReasonsConsistency = empiricalProfileCoverageRows.every((row) => {
    const reasons = toStringArray(row.empirical_release_gate_reasons, 160);
    const pass = row.empirical_release_gate_pass === true;
    return pass === (reasons.length === 0);
  });
  const empiricalCoverageGateExecutionChecksFailedConsistency = empiricalProfileCoverageRows.every(
    (row) => {
      const checksFailed = toStringArray(row.empirical_release_gate_checks_failed, 160);
      const executionOk = row.empirical_release_gate_execution_ok === true;
      return executionOk === (checksFailed.length === 0);
    },
  );
  const empiricalRequiredSourceRowsMissingParity = empiricalProfileCoverageRows.every((row) => {
    const derived = uniqueStringValues(
      (Array.isArray(row.empirical_required_source_rows) ? row.empirical_required_source_rows : [])
        .filter((source: any) => source?.required_missing === true)
        .map((source: any) => cleanText(source?.id || '', 80))
        .filter(Boolean),
    )
      .slice()
      .sort()
      .join(',');
    const declared = uniqueStringValues(
      toStringArray(row.empirical_required_sources_missing, 120),
    )
      .slice()
      .sort()
      .join(',');
    return derived === declared;
  });
  const empiricalRequiredMetricRowsMissingParity = empiricalProfileCoverageRows.every((row) => {
    const derived = uniqueStringValues(
      (Array.isArray(row.empirical_required_metric_rows) ? row.empirical_required_metric_rows : [])
        .filter((metric: any) => metric?.required_missing === true)
        .map((metric: any) => cleanText(metric?.key || '', 120))
        .filter(Boolean),
    )
      .slice()
      .sort()
      .join(',');
    const declared = uniqueStringValues(
      toStringArray(row.empirical_required_metrics_missing, 120),
    )
      .slice()
      .sort()
      .join(',');
    return derived === declared;
  });
  const empiricalRequiredMetricRowsNonPositiveParity = empiricalProfileCoverageRows.every((row) => {
    const derived = uniqueStringValues(
      (Array.isArray(row.empirical_required_metric_rows) ? row.empirical_required_metric_rows : [])
        .filter((metric: any) => metric?.required_non_positive === true)
        .map((metric: any) => cleanText(metric?.key || '', 120))
        .filter(Boolean),
    )
      .slice()
      .sort()
      .join(',');
    const declared = uniqueStringValues(
      toStringArray(row.empirical_required_positive_metrics_missing, 120),
    )
      .slice()
      .sort()
      .join(',');
    return derived === declared;
  });
  const empiricalProfileGateProfilesUnique =
    empiricalProfileGateProfileIds.length === new Set(empiricalProfileGateProfileIds).size;
  const empiricalProfileGateSummaryPartitionCount =
    empiricalProfileGateSummaryTotal ===
    empiricalProfileGateSummaryPassed + empiricalProfileGateSummaryFailed;
  const empiricalProfileGateSummaryFailingProfilesUnique =
    empiricalProfileGateFailingFromSummary.length ===
    new Set(empiricalProfileGateFailingFromSummary).size;
  const empiricalProfileGateSummaryFailingProfilesSubset =
    empiricalProfileGateFailingFromSummary.every((profileId) =>
      empiricalProfileGateProfileIds.includes(profileId),
    );
  const empiricalProfileReadinessProfilesUnique =
    empiricalProfileReadinessProfileIds.length ===
    new Set(empiricalProfileReadinessProfileIds).size;
  const empiricalProfileReadinessSummaryPartitionCount =
    empiricalProfileReadinessSummaryTotal ===
    empiricalProfileReadinessSummaryReleaseReady +
      empiricalProfileReadinessSummaryDegraded +
      empiricalProfileReadinessSummaryBlocked;
  const empiricalProfileReadinessScoreRange = empiricalProfileReadinessProfiles.every((row: any) => {
    const score = safeNumber(row?.readiness_score, -1);
    return Number.isFinite(score) && score >= 0 && score <= 100;
  });
  const empiricalProfileReadinessBlockedRequiresSevereReasons = empiricalProfileReadinessProfiles.every(
    (row: any) => {
      const readinessClass = cleanText(row?.readiness_class || '', 40);
      const severeReasons = toStringArray(row?.severe_reasons, 160);
      return readinessClass !== 'blocked' || severeReasons.length > 0;
    },
  );
  const empiricalProfileReadinessReleaseReadyReasonless = empiricalProfileReadinessProfiles.every(
    (row: any) => {
      const readinessClass = cleanText(row?.readiness_class || '', 40);
      const reasons = toStringArray(row?.reasons, 160);
      const severeReasons = toStringArray(row?.severe_reasons, 160);
      return readinessClass !== 'release_ready' || (reasons.length === 0 && severeReasons.length === 0);
    },
  );
  const empiricalMinimumContractProfileIds = Array.isArray(empiricalMinimumContract?.profiles)
    ? empiricalMinimumContract.profiles
        .map((row: any) => cleanText(row?.profile || '', 40))
        .filter(Boolean)
    : [];
  const empiricalMinimumContractProfileSetAlignment =
    uniqueStringValues(empiricalMinimumContractProfileIds).slice().sort().join(',') ===
    uniqueStringValues(empiricalProfileCoverageProfileIds).slice().sort().join(',');
  const empiricalMinimumContractMissingProfileIds = toStringArray(
    empiricalMinimumContract?.summary?.missing_profile_ids,
    40,
  );
  const empiricalMinimumContractMissingProfileIdsSubset =
    empiricalMinimumContractMissingProfileIds.every((profileId) =>
      empiricalMinimumContractProfileIds.includes(profileId),
    );
  const empiricalMinimumContractSummaryCountsParity = (() => {
    const profilesTotal = safeNumber(empiricalMinimumContract?.summary?.profiles_total, -1);
    const configuredProfiles = safeNumber(empiricalMinimumContract?.summary?.configured_profiles, -1);
    const missingProfiles = safeNumber(empiricalMinimumContract?.summary?.missing_profiles, -1);
    const minFloor = safeNumber(empiricalMinimumContract?.summary?.min_floor, -1);
    const maxFloor = safeNumber(empiricalMinimumContract?.summary?.max_floor, -1);
    return (
      profilesTotal === empiricalMinimumContractProfileIds.length &&
      profilesTotal === configuredProfiles + missingProfiles &&
      minFloor >= 0 &&
      maxFloor >= 0 &&
      minFloor <= maxFloor
    );
  })();
  contractChecks = contractChecks.concat([
    {
      id: 'runtime_proof_verify_empirical_coverage_release_gate_reasons_token_contract_v7',
      ok: empiricalCoverageReleaseGateReasonsTokenValid,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_coverage_release_gate_reasons_unique_contract_v7',
      ok: empiricalCoverageReleaseGateReasonsUnique,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_coverage_checks_failed_reason_parity_contract_v7',
      ok: empiricalCoverageChecksFailedReasonParity,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_coverage_gate_pass_reasons_consistency_contract_v7',
      ok: empiricalCoverageGatePassReasonsConsistency,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_coverage_gate_execution_checks_failed_consistency_contract_v7',
      ok: empiricalCoverageGateExecutionChecksFailedConsistency,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_required_source_rows_missing_parity_contract_v7',
      ok: empiricalRequiredSourceRowsMissingParity,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_required_metric_rows_missing_parity_contract_v7',
      ok: empiricalRequiredMetricRowsMissingParity,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_required_metric_rows_non_positive_parity_contract_v7',
      ok: empiricalRequiredMetricRowsNonPositiveParity,
      detail: `coverage_rows=${empiricalProfileCoverageRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_profiles_unique_contract_v7',
      ok: empiricalProfileGateProfilesUnique,
      detail: `gate_profiles=${empiricalProfileGateProfileIds.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_summary_partition_count_contract_v7',
      ok: empiricalProfileGateSummaryPartitionCount,
      detail: `summary_total=${empiricalProfileGateSummaryTotal};summary_passed=${empiricalProfileGateSummaryPassed};summary_failed=${empiricalProfileGateSummaryFailed}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_summary_failing_profiles_unique_contract_v7',
      ok: empiricalProfileGateSummaryFailingProfilesUnique,
      detail: `summary_failing_profiles=${empiricalProfileGateFailingFromSummary.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_summary_failing_profiles_subset_contract_v7',
      ok: empiricalProfileGateSummaryFailingProfilesSubset,
      detail: `summary_failing_profiles=${empiricalProfileGateFailingFromSummary.length};gate_profiles=${empiricalProfileGateProfileIds.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_readiness_profiles_unique_contract_v7',
      ok: empiricalProfileReadinessProfilesUnique,
      detail: `readiness_profiles=${empiricalProfileReadinessProfileIds.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_readiness_summary_partition_count_contract_v7',
      ok: empiricalProfileReadinessSummaryPartitionCount,
      detail: `summary_total=${empiricalProfileReadinessSummaryTotal};release_ready=${empiricalProfileReadinessSummaryReleaseReady};degraded=${empiricalProfileReadinessSummaryDegraded};blocked=${empiricalProfileReadinessSummaryBlocked}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_readiness_score_range_contract_v7',
      ok: empiricalProfileReadinessScoreRange,
      detail: `readiness_profiles=${empiricalProfileReadinessProfileIds.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_readiness_blocked_requires_severe_reasons_contract_v7',
      ok: empiricalProfileReadinessBlockedRequiresSevereReasons,
      detail: `readiness_profiles=${empiricalProfileReadinessProfileIds.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_readiness_release_ready_reasonless_contract_v7',
      ok: empiricalProfileReadinessReleaseReadyReasonless,
      detail: `readiness_profiles=${empiricalProfileReadinessProfileIds.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_minimum_contract_profile_set_alignment_contract_v7',
      ok: empiricalMinimumContractProfileSetAlignment,
      detail: `minimum_profiles=${empiricalMinimumContractProfileIds.length};coverage_profiles=${empiricalProfileCoverageProfileIds.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_minimum_contract_missing_profile_ids_subset_contract_v7',
      ok: empiricalMinimumContractMissingProfileIdsSubset,
      detail: `missing_profile_ids=${empiricalMinimumContractMissingProfileIds.length};minimum_profiles=${empiricalMinimumContractProfileIds.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_minimum_contract_summary_counts_parity_contract_v7',
      ok: empiricalMinimumContractSummaryCountsParity,
      detail: `minimum_profiles=${empiricalMinimumContractProfileIds.length}`,
    },
  ]);
  const empiricalProfileGateFailureRows = Array.isArray(empiricalProfileGateFailures?.failures)
    ? empiricalProfileGateFailures.failures
    : [];
  const empiricalProfileGateFailureProfileIds = empiricalProfileGateFailureRows
    .map((row: any) => cleanText(row?.profile || '', 40))
    .filter(Boolean);
  const empiricalProfileGateFailureProfilesTokenValid = empiricalProfileGateFailureProfileIds.every(
    (profileId) => /^[a-z0-9-]+$/.test(profileId),
  );
  const empiricalProfileGateFailureProfilesUnique =
    empiricalProfileGateFailureProfileIds.length ===
    new Set(empiricalProfileGateFailureProfileIds).size;
  const empiricalProfileGateFailureProfilesSubsetGate = empiricalProfileGateFailureProfileIds.every(
    (profileId) => empiricalProfileGateProfileIds.includes(profileId),
  );
  const empiricalProfileGateFailuresCountParity =
    safeNumber(empiricalProfileGateFailures?.failing_profiles_count, -1) ===
    empiricalProfileGateFailureRows.length;
  const empiricalProfileGateHasFailuresFlagParity =
    (empiricalProfileGateFailures?.has_failures === true) ===
    (empiricalProfileGateFailureRows.length > 0);
  const empiricalProfileGateFailureReasonsTokenValid = empiricalProfileGateFailureRows.every(
    (row: any) =>
      toStringArray(row?.reasons, 160).every((reason) =>
        /^[a-z0-9:_-]+$/.test(cleanText(reason, 120)),
      ),
  );
  const empiricalProfileGateFailureReasonsUnique = empiricalProfileGateFailureRows.every(
    (row: any) => {
      const reasons = toStringArray(row?.reasons, 160);
      return reasons.length === new Set(reasons).size;
    },
  );
  const empiricalProfileGateFailureChecksFailedTokenValid = empiricalProfileGateFailureRows.every(
    (row: any) =>
      toStringArray(row?.release_gate_checks_failed, 160).every((checkId) =>
        /^[a-z0-9:_-]+$/.test(cleanText(checkId, 120)),
      ),
  );
  const empiricalProfileGateFailureChecksFailedUnique = empiricalProfileGateFailureRows.every(
    (row: any) => {
      const checksFailed = toStringArray(row?.release_gate_checks_failed, 160);
      return checksFailed.length === new Set(checksFailed).size;
    },
  );
  const empiricalProfileGateFailureRequiredSourcesMissingTokenValid =
    empiricalProfileGateFailureRows.every((row: any) =>
      toStringArray(row?.required_sources_missing, 120).every((sourceId) =>
        /^[a-z0-9:_-]+$/.test(cleanText(sourceId, 120)),
      ),
    );
  const empiricalProfileGateFailureRequiredSourcesMissingUnique =
    empiricalProfileGateFailureRows.every((row: any) => {
      const sourceIds = toStringArray(row?.required_sources_missing, 120);
      return sourceIds.length === new Set(sourceIds).size;
    });
  const empiricalProfileGateFailureRequiredMetricsMissingTokenValid =
    empiricalProfileGateFailureRows.every((row: any) =>
      toStringArray(row?.required_metrics_missing, 120).every((metricKey) =>
        /^[a-z0-9:_-]+$/.test(cleanText(metricKey, 120)),
      ),
    );
  const empiricalProfileGateFailureRequiredMetricsMissingUnique =
    empiricalProfileGateFailureRows.every((row: any) => {
      const metricKeys = toStringArray(row?.required_metrics_missing, 120);
      return metricKeys.length === new Set(metricKeys).size;
    });
  const empiricalProfileGateFailureRequiredPositiveMetricsMissingTokenValid =
    empiricalProfileGateFailureRows.every((row: any) =>
      toStringArray(row?.required_positive_metrics_missing, 120).every(
        (metricKey) => /^[a-z0-9:_-]+$/.test(cleanText(metricKey, 120)),
      ),
    );
  const empiricalProfileGateFailureRequiredPositiveMetricsMissingUnique =
    empiricalProfileGateFailureRows.every((row: any) => {
      const metricKeys = toStringArray(row?.required_positive_metrics_missing, 120);
      return metricKeys.length === new Set(metricKeys).size;
    });
  const empiricalProfileGateByProfileId = new Map<string, any>(
    empiricalProfileGateProfiles.map((row: any) => [
      cleanText(row?.profile || '', 40),
      row,
    ]),
  );
  const empiricalProfileGateFailureReasonParityWithGate = empiricalProfileGateFailureRows.every(
    (row: any) => {
      const profileId = cleanText(row?.profile || '', 40);
      const gateRow = empiricalProfileGateByProfileId.get(profileId) || {};
      const failureReasons = uniqueStringValues(toStringArray(row?.reasons, 160))
        .slice()
        .sort()
        .join(',');
      const gateReasons = uniqueStringValues(
        toStringArray(gateRow?.empirical_release_gate_reasons, 160),
      )
        .slice()
        .sort()
        .join(',');
      return failureReasons === gateReasons;
    },
  );
  const empiricalProfileGateFailureChecksFailedParityWithGate =
    empiricalProfileGateFailureRows.every((row: any) => {
      const profileId = cleanText(row?.profile || '', 40);
      const gateRow = empiricalProfileGateByProfileId.get(profileId) || {};
      const failureChecks = uniqueStringValues(
        toStringArray(row?.release_gate_checks_failed, 160),
      )
        .slice()
        .sort()
        .join(',');
      const gateChecks = uniqueStringValues(
        toStringArray(gateRow?.empirical_release_gate_checks_failed, 160),
      )
        .slice()
        .sort()
        .join(',');
      return failureChecks === gateChecks;
    });
  const empiricalProfileGateFailureArtifactPathsTokenValid =
    empiricalProfileGateFailureRows.every((row: any) =>
      isCanonicalArtifactToken(cleanText(row?.source_artifact || '', 260)) &&
      isCanonicalArtifactToken(cleanText(row?.release_gate_artifact || '', 260)),
    );
  const empiricalMinimumContractSummaryMissingIdsCountParity =
    safeNumber(empiricalMinimumContract?.summary?.missing_profiles, -1) ===
    empiricalMinimumContractMissingProfileIds.length;
  const empiricalMinimumContractFloorExtremaParity = (() => {
    const rows = Array.isArray(empiricalMinimumContract?.profiles)
      ? empiricalMinimumContract.profiles
      : [];
    const requiredFloors = rows.map((row: any) =>
      safeNumber(row?.empirical_min_sample_points_required, 0),
    );
    const minDerived = requiredFloors.length > 0 ? Math.min(...requiredFloors) : 0;
    const maxDerived = requiredFloors.length > 0 ? Math.max(...requiredFloors) : 0;
    const minSummary = safeNumber(empiricalMinimumContract?.summary?.min_floor, -1);
    const maxSummary = safeNumber(empiricalMinimumContract?.summary?.max_floor, -1);
    return minSummary === minDerived && maxSummary === maxDerived;
  })();
  contractChecks = contractChecks.concat([
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_profiles_token_contract_v8',
      ok: empiricalProfileGateFailureProfilesTokenValid,
      detail: `failure_profiles=${empiricalProfileGateFailureProfileIds.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_profiles_unique_contract_v8',
      ok: empiricalProfileGateFailureProfilesUnique,
      detail: `failure_profiles=${empiricalProfileGateFailureProfileIds.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_profiles_subset_gate_contract_v8',
      ok: empiricalProfileGateFailureProfilesSubsetGate,
      detail: `failure_profiles=${empiricalProfileGateFailureProfileIds.length};gate_profiles=${empiricalProfileGateProfileIds.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_count_parity_contract_v8',
      ok: empiricalProfileGateFailuresCountParity,
      detail: `summary_failed=${safeNumber(empiricalProfileGateFailures?.failing_profiles_count, -1)};failure_rows=${empiricalProfileGateFailureRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_has_failures_flag_parity_contract_v8',
      ok: empiricalProfileGateHasFailuresFlagParity,
      detail: `has_failures=${empiricalProfileGateFailures?.has_failures === true};failure_rows=${empiricalProfileGateFailureRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_reasons_token_contract_v8',
      ok: empiricalProfileGateFailureReasonsTokenValid,
      detail: `failure_rows=${empiricalProfileGateFailureRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_reasons_unique_contract_v8',
      ok: empiricalProfileGateFailureReasonsUnique,
      detail: `failure_rows=${empiricalProfileGateFailureRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_checks_failed_token_contract_v8',
      ok: empiricalProfileGateFailureChecksFailedTokenValid,
      detail: `failure_rows=${empiricalProfileGateFailureRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_checks_failed_unique_contract_v8',
      ok: empiricalProfileGateFailureChecksFailedUnique,
      detail: `failure_rows=${empiricalProfileGateFailureRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_required_sources_missing_token_contract_v8',
      ok: empiricalProfileGateFailureRequiredSourcesMissingTokenValid,
      detail: `failure_rows=${empiricalProfileGateFailureRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_required_sources_missing_unique_contract_v8',
      ok: empiricalProfileGateFailureRequiredSourcesMissingUnique,
      detail: `failure_rows=${empiricalProfileGateFailureRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_required_metrics_missing_token_contract_v8',
      ok: empiricalProfileGateFailureRequiredMetricsMissingTokenValid,
      detail: `failure_rows=${empiricalProfileGateFailureRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_required_metrics_missing_unique_contract_v8',
      ok: empiricalProfileGateFailureRequiredMetricsMissingUnique,
      detail: `failure_rows=${empiricalProfileGateFailureRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_required_positive_metrics_missing_token_contract_v8',
      ok: empiricalProfileGateFailureRequiredPositiveMetricsMissingTokenValid,
      detail: `failure_rows=${empiricalProfileGateFailureRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_required_positive_metrics_missing_unique_contract_v8',
      ok: empiricalProfileGateFailureRequiredPositiveMetricsMissingUnique,
      detail: `failure_rows=${empiricalProfileGateFailureRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_reason_parity_with_gate_contract_v8',
      ok: empiricalProfileGateFailureReasonParityWithGate,
      detail: `failure_rows=${empiricalProfileGateFailureRows.length};gate_profiles=${empiricalProfileGateProfileIds.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_checks_failed_parity_with_gate_contract_v8',
      ok: empiricalProfileGateFailureChecksFailedParityWithGate,
      detail: `failure_rows=${empiricalProfileGateFailureRows.length};gate_profiles=${empiricalProfileGateProfileIds.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_profile_gate_failures_artifact_paths_token_contract_v8',
      ok: empiricalProfileGateFailureArtifactPathsTokenValid,
      detail: `failure_rows=${empiricalProfileGateFailureRows.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_minimum_contract_summary_missing_ids_count_parity_contract_v8',
      ok: empiricalMinimumContractSummaryMissingIdsCountParity,
      detail: `summary_missing_profiles=${safeNumber(empiricalMinimumContract?.summary?.missing_profiles, -1)};missing_ids=${empiricalMinimumContractMissingProfileIds.length}`,
    },
    {
      id: 'runtime_proof_verify_empirical_minimum_contract_floor_extrema_parity_contract_v8',
      ok: empiricalMinimumContractFloorExtremaParity,
      detail: `minimum_profiles=${empiricalMinimumContractProfileIds.length}`,
    },
  ]);
  const contractFailures = contractChecks
    .filter((row) => !row.ok)
    .map((row) => ({ id: row.id, detail: row.detail }));
  const failures = baseFailures.concat(contractFailures);
  const ok =
    profileRuns.every((row) => row.ok) &&
    boundednessEvidence.ok &&
    boundednessProfilesEvidence.ok &&
    multiDaySoakEvidence.ok &&
    empiricalMinimumContract.ok &&
    empiricalProfileCoverage.ok &&
    empiricalProfileGate.ok &&
    empiricalProfileReadiness.ok &&
    proofChecksums.ok &&
    contractFailures.length === 0;

  const report = {
    ok,
    type: 'runtime_proof_verify',
    profile: args.profile,
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    summary: {
      pass: ok,
      proof_track: args.proofTrack,
      profile_count: summaryProfileCount,
      profiles_passed: summaryProfilesPassed,
      boundedness_inspect_exit_max: summaryBoundednessInspectExitMax,
    },
    profile_runs: profileRunRows,
    evidence,
    contracts: {
      pass: contractFailures.length === 0,
      check_count: contractChecks.length,
      failed_count: contractFailures.length,
      failed_ids: contractFailures.map((row) => row.id),
      checks: contractChecks,
      detail: {
        expected_profiles: expectedProfilesForSelection,
        observed_profiles: observedProfiles,
        artifact_path_count: artifactPaths.length,
        evidence_path_count: evidencePaths.length,
        checksum_row_count: checksumRows.length,
        base_failure_count: baseFailures.length,
      },
    },
    artifact_paths: artifactPaths,
    failures,
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
