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
  return {
    strict: common.strict,
    profile,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    policyPath: cleanText(readFlag(argv, 'policy') || 'tests/tooling/config/release_gates.yaml', 400),
    metricsPath: cleanText(
      readFlag(argv, 'metrics') || 'core/local/artifacts/runtime_proof_metrics_rich_current.json',
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

function safeNumber(value: unknown, fallback = 0): number {
  const num = Number(value);
  return Number.isFinite(num) ? num : fallback;
}

function statusForUtilization(value: number): 'healthy' | 'warning' | 'critical' {
  if (value >= 1) return 'critical';
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

  const metricsPayload = readJsonMaybe(args.metricsPath) || {};
  const metrics = metricsPayload.metrics || metricsPayload;
  const benchmarkPayload = readJsonMaybe(args.benchmarkPath) || {};
  const queuePolicy = readJsonMaybe(args.queuePolicyPath) || {};

  const profileLabel = args.profile === 'tiny-max' ? 'InfRing (tiny-max)' : `InfRing (${args.profile})`;
  const benchmarkInstallSize = safeNumber(
    benchmarkPayload?.projects?.[profileLabel]?.install_size_mb ??
      benchmarkPayload?.medians?.[args.profile]?.install_size_mb ??
      benchmarkPayload?.[args.profile]?.install_size_mb,
    0,
  );
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
      actual: Number(metrics.peak_rss_mb || 0),
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
    artifact_paths: [args.markdownOutPath],
  };

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
