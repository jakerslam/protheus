#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type ProfileId = 'rich' | 'pure' | 'tiny-max';

type ProfileGatePolicy = {
  memory: { peak_rss_mb_max: number };
  storage: { disk_mb_max: number };
  queue: { depth_max: number; depth_p95_max: number };
  receipts: { throughput_per_min_min: number; p95_latency_ms_max: number };
  recovery: {
    conduit_recovery_ms_max: number;
    adapter_restart_count_max: number;
    adapter_recovery_ms_max: number;
  };
  stale_surface: { incidents_max: number };
};

type ParsedPolicy = {
  version: number;
  profiles: Record<string, ProfileGatePolicy>;
};

function parseProfile(raw: string | undefined): ProfileId | null {
  const normalized = cleanText(raw || 'rich', 32).toLowerCase();
  if (normalized === 'rich') return 'rich';
  if (normalized === 'pure') return 'pure';
  if (normalized === 'tiny-max' || normalized === 'tiny' || normalized === 'tiny_max') return 'tiny-max';
  return null;
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/runtime_boundedness_inspect_current.json',
  });
  const profile = parseProfile(readFlag(argv, 'profile'));
  const profileSuffix = profile || 'unknown';
  return {
    strict: common.strict,
    profile,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    policyPath: cleanText(readFlag(argv, 'policy') || 'tests/tooling/config/release_gates.yaml', 400),
    metricsPath: cleanText(
      readFlag(argv, 'metrics') ||
        `core/local/artifacts/runtime_proof_release_gate_${profileSuffix}_current.json`,
      400,
    ),
    benchmarkPath: cleanText(
      readFlag(argv, 'benchmark') || 'docs/client/reports/benchmark_matrix_run_latest.json',
      400,
    ),
    queuePolicyPath: cleanText(
      readFlag(argv, 'queue-policy') || 'client/runtime/config/queue_backpressure_policy.json',
      400,
    ),
    markdownOutPath: cleanText(
      readFlag(argv, 'out-markdown') || 'local/workspace/reports/RUNTIME_BOUNDEDNESS_INSPECT_CURRENT.md',
      400,
    ),
    boundednessReportOutPath: cleanText(
      readFlag(argv, 'out-boundedness-report') ||
        `core/local/artifacts/runtime_boundedness_report_${profileSuffix}_current.json`,
      400,
    ),
  };
}

function parsePolicyYaml(raw: string): ParsedPolicy {
  const policy: ParsedPolicy = { version: 1, profiles: {} };
  let currentProfile = '';
  let currentSection = '';
  for (const lineRaw of raw.split('\n')) {
    const line = lineRaw.replace(/\t/g, '    ');
    const noComment = line.split('#')[0] || '';
    const trimmed = noComment.trim();
    if (!trimmed) continue;

    const versionMatch = /^version:\s*(\d+)\s*$/.exec(trimmed);
    if (versionMatch) {
      policy.version = Number(versionMatch[1] || 1);
      continue;
    }

    const profileMatch = /^ {2}([a-zA-Z0-9_-]+):\s*$/.exec(noComment);
    if (profileMatch) {
      currentProfile = cleanText(profileMatch[1], 60);
      currentSection = '';
      if (!policy.profiles[currentProfile]) {
        policy.profiles[currentProfile] = {
          memory: { peak_rss_mb_max: 0 },
          storage: { disk_mb_max: 0 },
          queue: { depth_max: 0, depth_p95_max: 0 },
          receipts: { throughput_per_min_min: 0, p95_latency_ms_max: 0 },
          recovery: {
            conduit_recovery_ms_max: 0,
            adapter_restart_count_max: 0,
            adapter_recovery_ms_max: 0,
          },
          stale_surface: { incidents_max: 0 },
        };
      }
      continue;
    }

    const sectionMatch = /^ {4}([a-zA-Z0-9_-]+):\s*$/.exec(noComment);
    if (sectionMatch) {
      currentSection = cleanText(sectionMatch[1], 60);
      continue;
    }

    const kvMatch = /^ {6}([a-zA-Z0-9_.-]+):\s*([-+]?[0-9]*\.?[0-9]+)\s*$/.exec(noComment);
    if (kvMatch && currentProfile && currentSection) {
      const key = cleanText(kvMatch[1], 80);
      const value = Number(kvMatch[2]);
      const row = policy.profiles[currentProfile] as any;
      if (row[currentSection] && Object.prototype.hasOwnProperty.call(row[currentSection], key)) {
        row[currentSection][key] = value;
      }
    }
  }
  return policy;
}

function ratio(actual: number, limit: number): number {
  if (!Number.isFinite(actual) || !Number.isFinite(limit) || limit <= 0) return 0;
  return Math.round((actual / limit) * 1000) / 1000;
}

function pct(value: number): number {
  return Math.round(value * 10000) / 100;
}

function readJsonMaybe(relPath: string): any {
  const abs = path.resolve(relPath);
  if (!fs.existsSync(abs)) return null;
  return JSON.parse(fs.readFileSync(abs, 'utf8'));
}

function extractMetricsPayload(metricsPayload: any): { metrics: Record<string, any>; source: string } {
  if (!metricsPayload || typeof metricsPayload !== 'object') {
    return { metrics: {}, source: 'missing' };
  }

  if (metricsPayload?.type === 'runtime_proof_release_gate') {
    const effectiveMetrics = metricsPayload?.effective_metrics?.metrics;
    if (effectiveMetrics && typeof effectiveMetrics === 'object') {
      return {
        metrics: effectiveMetrics,
        source: 'runtime_proof_release_gate.effective_metrics',
      };
    }
    const legacyMetrics = metricsPayload?.metrics;
    if (legacyMetrics && typeof legacyMetrics === 'object') {
      return {
        metrics: legacyMetrics,
        source: 'runtime_proof_release_gate.metrics',
      };
    }
  }

  if (metricsPayload?.type === 'runtime_proof_release_metrics') {
    const releaseMetrics = metricsPayload?.metrics;
    if (releaseMetrics && typeof releaseMetrics === 'object') {
      return {
        metrics: releaseMetrics,
        source: 'runtime_proof_release_metrics.metrics',
      };
    }
  }

  if (metricsPayload?.type === 'runtime_proof_metrics') {
    const proofMetrics = metricsPayload?.metrics;
    if (proofMetrics && typeof proofMetrics === 'object') {
      return {
        metrics: proofMetrics,
        source: 'runtime_proof_metrics.metrics',
      };
    }
  }

  if (metricsPayload?.metrics && typeof metricsPayload.metrics === 'object') {
    return {
      metrics: metricsPayload.metrics,
      source: 'generic.metrics',
    };
  }

  return {
    metrics: metricsPayload,
    source: 'raw_payload',
  };
}

function safeNumber(value: unknown, fallback = 0): number {
  const num = Number(value);
  return Number.isFinite(num) ? num : fallback;
}

function isCanonicalToken(raw: string, maxLen = 120): boolean {
  const token = cleanText(String(raw || ''), maxLen);
  return /^[a-z0-9][a-z0-9._:-]*$/i.test(token);
}

function isCanonicalPathToken(raw: string, maxLen = 400): boolean {
  const token = cleanText(String(raw || ''), maxLen);
  if (!token) return false;
  if (/^\s|\s$/.test(String(raw || ''))) return false;
  return /^[a-z0-9_./:-]+$/i.test(token);
}

function isFiniteNonNegative(value: unknown): boolean {
  const num = Number(value);
  return Number.isFinite(num) && num >= 0;
}

function nearlyEqual(a: number, b: number, epsilon = 0.01): boolean {
  return Math.abs(a - b) <= epsilon;
}

function statusForUtilization(value: number): 'healthy' | 'warning' | 'critical' {
  if (value > 1) return 'critical';
  if (value >= 0.85) return 'warning';
  return 'healthy';
}

type QueueBand = {
  id: string;
  action: string;
  min_utilization?: number;
  max_utilization?: number;
};

function selectQueueBand(queuePolicy: any, utilization: number): QueueBand | null {
  const bands = Array.isArray(queuePolicy?.utilization_bands) ? queuePolicy.utilization_bands : [];
  for (const raw of bands) {
    const band: QueueBand = {
      id: cleanText(String(raw?.id || ''), 40),
      action: cleanText(String(raw?.action || ''), 80),
      min_utilization: raw?.min_utilization == null ? undefined : safeNumber(raw?.min_utilization, NaN),
      max_utilization: raw?.max_utilization == null ? undefined : safeNumber(raw?.max_utilization, NaN),
    };
    if (!band.id || !band.action) continue;
    const minOk =
      band.min_utilization == null || Number.isNaN(band.min_utilization)
        ? true
        : utilization >= band.min_utilization;
    const maxOk =
      band.max_utilization == null || Number.isNaN(band.max_utilization)
        ? true
        : utilization <= band.max_utilization;
    if (minOk && maxOk) {
      return band;
    }
  }
  return null;
}

function markdown(report: any): string {
  const lines = [
    '# Runtime Boundedness Inspect',
    '',
    `- profile: ${report.profile}`,
    `- policy: ${report.policy_path}`,
    `- metrics: ${report.metrics_path}`,
    `- overall_status: ${report.summary.overall_status}`,
    '',
    '| metric | actual | limit | utilization | status |',
    '| --- | ---: | ---: | ---: | --- |',
  ];
  for (const row of report.rows) {
    lines.push(
      `| ${row.metric} | ${row.actual} | ${row.limit} | ${row.utilization_pct}% | ${row.status} |`,
    );
  }
  lines.push('');
  lines.push('## Controllers');
  lines.push('- receipt_retention_window_hours: 72');
  lines.push('- stale_surface_ttl_minutes: 30');
  lines.push('- compaction_trigger_queue_depth_ratio: 0.85');
  lines.push('- eviction_strategy: least_recent_receipt_first');
  lines.push(`- queue_backpressure_band: ${report.controllers.queue_backpressure_band}`);
  lines.push(`- queue_backpressure_action: ${report.controllers.queue_backpressure_action}`);
  lines.push(`- queue_backpressure_policy_path: ${report.controllers.queue_backpressure_policy_path}`);
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);

  if (!args.profile) {
    const payload = {
      ok: false,
      type: 'runtime_boundedness_inspect',
      error: 'runtime_proof_profile_invalid',
      profile: cleanText(readFlag(argv, 'profile') || '', 40),
      allowed_profiles: ['rich', 'pure', 'tiny-max'],
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  const policyRaw = fs.readFileSync(path.resolve(root, args.policyPath), 'utf8');
  const policy = parsePolicyYaml(policyRaw);
  const profilePolicy = policy.profiles[args.profile];
  if (!profilePolicy) {
    const payload = {
      ok: false,
      type: 'runtime_boundedness_inspect',
      error: 'runtime_boundedness_policy_profile_missing',
      profile: args.profile,
      policy_path: args.policyPath,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }
  const failures: Array<{ id: string; detail: string }> = [];
  if (!isCanonicalToken(args.profile, 40)) {
    failures.push({
      id: 'runtime_boundedness_inspect_profile_token_contract_v2',
      detail: args.profile,
    });
  }
  if (!Number.isInteger(policy.version) || Number(policy.version) < 1) {
    failures.push({
      id: 'runtime_boundedness_inspect_policy_version_scalar_contract_v2',
      detail: `version=${String(policy.version)}`,
    });
  }
  const requiredSections = ['memory', 'storage', 'queue', 'receipts', 'recovery', 'stale_surface'];
  for (const section of requiredSections) {
    const sectionValue = (profilePolicy as any)?.[section];
    if (!sectionValue || typeof sectionValue !== 'object' || Array.isArray(sectionValue)) {
      failures.push({
        id: 'runtime_boundedness_inspect_policy_profile_sections_present_contract_v2',
        detail: `${args.profile}:${section}`,
      });
    }
  }
  const policyThresholds: Array<[string, unknown]> = [
    ['memory.peak_rss_mb_max', profilePolicy?.memory?.peak_rss_mb_max],
    ['storage.disk_mb_max', profilePolicy?.storage?.disk_mb_max],
    ['queue.depth_max', profilePolicy?.queue?.depth_max],
    ['queue.depth_p95_max', profilePolicy?.queue?.depth_p95_max],
    ['receipts.throughput_per_min_min', profilePolicy?.receipts?.throughput_per_min_min],
    ['receipts.p95_latency_ms_max', profilePolicy?.receipts?.p95_latency_ms_max],
    ['recovery.conduit_recovery_ms_max', profilePolicy?.recovery?.conduit_recovery_ms_max],
    ['recovery.adapter_restart_count_max', profilePolicy?.recovery?.adapter_restart_count_max],
    ['recovery.adapter_recovery_ms_max', profilePolicy?.recovery?.adapter_recovery_ms_max],
    ['stale_surface.incidents_max', profilePolicy?.stale_surface?.incidents_max],
  ];
  for (const [key, value] of policyThresholds) {
    if (!isFiniteNonNegative(value)) {
      failures.push({
        id: 'runtime_boundedness_inspect_policy_threshold_non_negative_contract_v2',
        detail: `${args.profile}:${key}:${String(value)}`,
      });
    }
  }

  const metricsPayload = readJsonMaybe(args.metricsPath) || {};
  const metricSelection = extractMetricsPayload(metricsPayload);
  const metrics = metricSelection.metrics;
  const benchmarkPayload = readJsonMaybe(args.benchmarkPath) || {};
  const queuePolicy = readJsonMaybe(args.queuePolicyPath) || {};

  const profileLabel = args.profile === 'tiny-max' ? 'InfRing (tiny-max)' : `InfRing (${args.profile})`;
  const benchmarkMeasured =
    args.profile === 'rich'
      ? benchmarkPayload?.infring_measured
      : args.profile === 'pure'
        ? benchmarkPayload?.pure_workspace_measured
        : benchmarkPayload?.pure_workspace_tiny_max_measured;
  const benchmarkIdleMemory = safeNumber(
    benchmarkMeasured?.idle_memory_mb ??
      benchmarkPayload?.projects?.[profileLabel]?.idle_memory_mb ??
      benchmarkPayload?.medians?.[args.profile]?.idle_memory_mb ??
      benchmarkPayload?.[args.profile]?.idle_memory_mb,
    0,
  );
  const benchmarkInstallSize = safeNumber(
    benchmarkPayload?.projects?.[profileLabel]?.install_size_mb ??
      benchmarkPayload?.medians?.[args.profile]?.install_size_mb ??
      benchmarkPayload?.[args.profile]?.install_size_mb,
    0,
  );
  const measuredPeakRss = safeNumber(metrics.peak_rss_mb, 0);
  const peakRssActual = measuredPeakRss > 0 ? measuredPeakRss : benchmarkIdleMemory;
  const storageActual = safeNumber(
    metrics.storage_usage_mb ??
      metrics.install_size_mb ??
      metrics.artifact_size_mb ??
      benchmarkInstallSize,
    0,
  );

  const rowSet = [
    {
      metric: 'peak_rss_mb',
      actual: peakRssActual,
      limit: Number(profilePolicy.memory.peak_rss_mb_max || 0),
    },
    {
      metric: 'storage_usage_mb',
      actual: storageActual,
      limit: Number(profilePolicy.storage.disk_mb_max || 0),
    },
    {
      metric: 'queue_depth_max',
      actual: Number(metrics.queue_depth_max || 0),
      limit: Number(profilePolicy.queue.depth_max || 0),
    },
    {
      metric: 'queue_depth_p95',
      actual: Number(metrics.queue_depth_p95 || 0),
      limit: Number(profilePolicy.queue.depth_p95_max || 0),
    },
    {
      metric: 'receipt_p95_latency_ms',
      actual: Number(metrics.receipt_p95_latency_ms || 0),
      limit: Number(profilePolicy.receipts.p95_latency_ms_max || 0),
    },
    {
      metric: 'conduit_recovery_ms',
      actual: Number(metrics.conduit_recovery_ms || 0),
      limit: Number(profilePolicy.recovery.conduit_recovery_ms_max || 0),
    },
    {
      metric: 'adapter_recovery_ms',
      actual: Number(metrics.adapter_recovery_ms || 0),
      limit: Number(profilePolicy.recovery.adapter_recovery_ms_max || 0),
    },
    {
      metric: 'stale_surface_incidents',
      actual: Number(metrics.stale_surface_incidents || 0),
      limit: Number(profilePolicy.stale_surface.incidents_max || 0),
    },
  ];

  const throughputActual = Number(metrics.receipt_throughput_per_min || 0);
  const throughputLimitMin = Number(profilePolicy.receipts.throughput_per_min_min || 0);

  const rows = rowSet.map((row) => {
    const utilization = ratio(row.actual, row.limit);
    return {
      ...row,
      utilization,
      utilization_pct: pct(utilization),
      status: statusForUtilization(utilization),
    };
  });

  const throughputOk = throughputActual >= throughputLimitMin;
  const throughputUtilization = throughputLimitMin > 0 ? ratio(throughputLimitMin, Math.max(throughputActual, 1)) : 0;
  rows.push({
    metric: 'receipt_throughput_per_min_min',
    actual: throughputActual,
    limit: throughputLimitMin,
    utilization: throughputUtilization,
    utilization_pct: pct(throughputUtilization),
    status: throughputOk ? 'healthy' : 'critical',
  });
  const expectedRowCount = rowSet.length + 1;
  if (rows.length !== expectedRowCount) {
    failures.push({
      id: 'runtime_boundedness_inspect_rows_count_expected_contract_v2',
      detail: `rows=${rows.length};expected=${expectedRowCount}`,
    });
  }
  const rowMetricTokens = rows.map((row) => cleanText(String(row?.metric || ''), 80));
  for (const metricToken of rowMetricTokens) {
    if (!isCanonicalToken(metricToken, 80)) {
      failures.push({
        id: 'runtime_boundedness_inspect_rows_metric_token_contract_v2',
        detail: metricToken || 'missing',
      });
    }
  }
  if (new Set(rowMetricTokens).size !== rowMetricTokens.length) {
    failures.push({
      id: 'runtime_boundedness_inspect_rows_metric_unique_contract_v2',
      detail: rowMetricTokens.join(','),
    });
  }
  for (const row of rows) {
    const metric = cleanText(String(row?.metric || ''), 80) || 'unknown';
    const numberFields = [
      ['actual', row?.actual],
      ['limit', row?.limit],
      ['utilization', row?.utilization],
      ['utilization_pct', row?.utilization_pct],
    ] as const;
    for (const [field, value] of numberFields) {
      if (!isFiniteNonNegative(value)) {
        failures.push({
          id: 'runtime_boundedness_inspect_rows_numeric_non_negative_contract_v2',
          detail: `${metric}:${field}=${String(value)}`,
        });
      }
    }
    if (!nearlyEqual(Number(row?.utilization_pct || 0), pct(Number(row?.utilization || 0)))) {
      failures.push({
        id: 'runtime_boundedness_inspect_rows_utilization_consistency_contract_v2',
        detail: `${metric}:utilization=${String(row?.utilization)};utilization_pct=${String(row?.utilization_pct)}`,
      });
    }
    const status = cleanText(String(row?.status || ''), 24);
    if (!['healthy', 'warning', 'critical'].includes(status)) {
      failures.push({
        id: 'runtime_boundedness_inspect_rows_status_token_contract_v2',
        detail: `${metric}:${status || 'missing'}`,
      });
    }
    if (metric !== 'receipt_throughput_per_min_min') {
      const expectedStatus = statusForUtilization(Number(row?.utilization || 0));
      if (status && status !== expectedStatus) {
        failures.push({
          id: 'runtime_boundedness_inspect_rows_status_threshold_contract_v2',
          detail: `${metric}:status=${status};expected=${expectedStatus}`,
        });
      }
    }
  }

  const statuses = rows.map((row) => row.status);
  const overallStatus = statuses.includes('critical')
    ? 'critical'
    : statuses.includes('warning')
    ? 'warning'
    : 'healthy';

  const queueDepthMaxRow = rows.find((row) => row.metric === 'queue_depth_max');
  const queueUtilization = safeNumber(queueDepthMaxRow?.utilization, 0);
  const queueBand = selectQueueBand(queuePolicy, queueUtilization);

  const failures = rows
    .filter((row) => row.status === 'critical')
    .map((row) => ({ id: 'boundedness_violation', detail: `${row.metric}:actual=${row.actual},limit=${row.limit}` }));
  if (!queueBand) {
    failures.push({
      id: 'queue_backpressure_policy_unresolved',
      detail: `utilization=${queueUtilization};policy=${args.queuePolicyPath}`,
    });
  }

  const report = {
    ok: failures.length === 0,
    type: 'runtime_boundedness_inspect',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    profile: args.profile,
    policy_path: args.policyPath,
    metrics_path: args.metricsPath,
    metrics_source: metricSelection.source,
    benchmark_path: args.benchmarkPath,
    queue_policy_path: args.queuePolicyPath,
    markdown_path: args.markdownOutPath,
    summary: {
      pass: failures.length === 0,
      row_count: rows.length,
      failure_count: failures.length,
      overall_status: overallStatus,
    },
    boundedness_contract: {
      rss_ceiling_mb: Number(profilePolicy.memory.peak_rss_mb_max || 0),
      storage_ceiling_mb: Number(profilePolicy.storage.disk_mb_max || 0),
      queue_depth_max_ceiling: Number(profilePolicy.queue.depth_max || 0),
      queue_depth_p95_ceiling: Number(profilePolicy.queue.depth_p95_max || 0),
      stale_surface_incidents_max: Number(profilePolicy.stale_surface.incidents_max || 0),
      conduit_recovery_ms_max: Number(profilePolicy.recovery.conduit_recovery_ms_max || 0),
      adapter_recovery_ms_max: Number(profilePolicy.recovery.adapter_recovery_ms_max || 0),
    },
    controllers: {
      receipt_retention_window_hours: 72,
      stale_surface_ttl_minutes: 30,
      compaction_trigger_queue_depth_ratio: 0.85,
      eviction_strategy: 'least_recent_receipt_first',
      queue_backpressure_policy_path: args.queuePolicyPath,
      queue_backpressure_utilization: queueUtilization,
      queue_backpressure_band: queueBand?.id || 'unresolved',
      queue_backpressure_action: queueBand?.action || 'unknown',
    },
    rows,
    failures,
    artifact_paths: [args.markdownOutPath, args.boundednessReportOutPath],
  };
  const recomputedOverallStatus = statuses.includes('critical')
    ? 'critical'
    : statuses.includes('warning')
      ? 'warning'
      : 'healthy';
  if (!['healthy', 'warning', 'critical'].includes(cleanText(String(report?.summary?.overall_status || ''), 24))) {
    failures.push({
      id: 'runtime_boundedness_inspect_overall_status_token_contract_v2',
      detail: cleanText(String(report?.summary?.overall_status || ''), 40) || 'missing',
    });
  }
  if (cleanText(String(report?.summary?.overall_status || ''), 24) !== recomputedOverallStatus) {
    failures.push({
      id: 'runtime_boundedness_inspect_overall_status_parity_contract_v2',
      detail: `summary=${String(report?.summary?.overall_status)};derived=${recomputedOverallStatus}`,
    });
  }
  if (Number(report?.summary?.row_count || 0) !== rows.length) {
    failures.push({
      id: 'runtime_boundedness_inspect_summary_row_count_parity_contract_v2',
      detail: `summary=${String(report?.summary?.row_count)};rows=${rows.length}`,
    });
  }
  if (Number(report?.summary?.failure_count || 0) !== failures.length) {
    failures.push({
      id: 'runtime_boundedness_inspect_summary_failure_count_parity_contract_v2',
      detail: `summary=${String(report?.summary?.failure_count)};failures=${failures.length}`,
    });
  }
  if (Boolean(report?.summary?.pass) !== (failures.length === 0)) {
    failures.push({
      id: 'runtime_boundedness_inspect_summary_pass_parity_contract_v2',
      detail: `summary_pass=${String(report?.summary?.pass)};derived=${String(failures.length === 0)}`,
    });
  }
  if ((queueBand ? 'resolved' : 'unresolved') !== report.controllers.queue_backpressure_band && !queueBand) {
    failures.push({
      id: 'runtime_boundedness_inspect_controller_band_resolution_contract_v2',
      detail: `band=${String(report.controllers.queue_backpressure_band)}`,
    });
  }
  if (queueBand && !isCanonicalToken(cleanText(String(report.controllers.queue_backpressure_action || ''), 80), 80)) {
    failures.push({
      id: 'runtime_boundedness_inspect_controller_action_nonempty_contract_v2',
      detail: String(report.controllers.queue_backpressure_action || 'missing'),
    });
  }
  const artifactPaths = Array.isArray(report.artifact_paths) ? report.artifact_paths : [];
  const artifactTokens = artifactPaths.map((row) => cleanText(String(row || ''), 400)).filter(Boolean);
  if (
    artifactTokens.length === 0 ||
    artifactTokens.length !== artifactPaths.length ||
    new Set(artifactTokens).size !== artifactTokens.length ||
    artifactTokens.some((token) => !isCanonicalPathToken(token, 400))
  ) {
    failures.push({
      id: 'runtime_boundedness_inspect_artifact_paths_nonempty_unique_contract_v2',
      detail: artifactTokens.join(',') || 'missing',
    });
  }
  const failureRows = [...failures];
  const failureIds = failureRows.map((row) => cleanText(String(row?.id || ''), 120)).filter(Boolean);
  if (
    failureRows.some(
      (row) =>
        !isCanonicalToken(cleanText(String(row?.id || ''), 120), 120) ||
        !cleanText(String(row?.detail || ''), 400),
    ) ||
    new Set(failureIds).size !== failureIds.length
  ) {
    failures.push({
      id: 'runtime_boundedness_inspect_failure_rows_shape_contract_v2',
      detail: failureIds.join(',') || 'missing',
    });
  }
  report.ok = failures.length === 0;
  report.summary.pass = failures.length === 0;
  report.summary.row_count = rows.length;
  report.summary.failure_count = failures.length;

  const metric = (id: string) => rows.find((row) => row.metric === id);
  const peakRss = safeNumber(metric('peak_rss_mb')?.actual, 0);
  const queueDepthMax = safeNumber(metric('queue_depth_max')?.actual, 0);
  const queueDepthP95 = safeNumber(metric('queue_depth_p95')?.actual, 0);
  const staleSurfaceCount = safeNumber(metric('stale_surface_incidents')?.actual, 0);
  const conduitRecoveryMs = safeNumber(metric('conduit_recovery_ms')?.actual, 0);
  const adapterRecoveryMs = safeNumber(metric('adapter_recovery_ms')?.actual, 0);
  const boundednessReport = {
    ok: report.ok,
    type: 'runtime_boundedness_report',
    generated_at: report.generated_at,
    revision: report.revision,
    profile: args.profile,
    source_inspect_artifact: args.outPath,
    summary: {
      pass: report.summary.pass,
      overall_status: report.summary.overall_status,
      max_rss_mb: peakRss,
      queue_depth_max: queueDepthMax,
      queue_depth_p95: queueDepthP95,
      stale_surface_count: staleSurfaceCount,
      recovery_time_ms_max: Math.max(conduitRecoveryMs, adapterRecoveryMs),
      recovery_time_ms_conduit: conduitRecoveryMs,
      recovery_time_ms_adapter: adapterRecoveryMs,
    },
  };
  const boundednessReportAbs = path.resolve(root, args.boundednessReportOutPath);
  fs.mkdirSync(path.dirname(boundednessReportAbs), { recursive: true });
  fs.writeFileSync(boundednessReportAbs, `${JSON.stringify(boundednessReport, null, 2)}\n`, 'utf8');
  writeTextArtifact(args.markdownOutPath, markdown(report));

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
