#!/usr/bin/env tsx

import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

type ProfileId = 'rich' | 'pure' | 'tiny-max';
type RefreshMode = 'auto' | 'always' | 'never';
type ProofTrackId = 'synthetic' | 'empirical' | 'dual';

type ProfileGatePolicy = {
  proof_tracks: {
    synthetic_required: number;
    empirical_required: number;
    empirical_min_sample_points: number;
  };
  memory: { peak_rss_mb_max: number };
  queue: { depth_max: number; depth_p95_max: number };
  receipts: { throughput_per_min_min: number; p95_latency_ms_max: number };
  recovery: {
    conduit_recovery_ms_max: number;
    adapter_restart_count_max: number;
    adapter_recovery_ms_max: number;
  };
  stale_surface: { incidents_max: number };
  adapter_chaos: {
    baseline_pass_ratio_min: number;
    fail_closed_ratio_min: number;
    graduation_ratio_min: number;
  };
  quality_telemetry: {
    empty_final_max: number;
    deferred_final_max: number;
    placeholder_final_max: number;
    off_topic_final_max: number;
    meta_status_tool_leak_max: number;
    web_missing_tool_attempt_max: number;
    taxonomy_parse_error_max: number;
  };
};

type ParsedPolicy = {
  version: number;
  profiles: Record<string, ProfileGatePolicy>;
};

type GateCheck = {
  id: string;
  comparator: '<=' | '>=';
  actual: number;
  threshold: number;
  ok: boolean;
  detail: string;
};

function parseProfile(raw: string | undefined): ProfileId | null {
  const normalized = cleanText(raw || 'rich', 32).toLowerCase();
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

function parseRefreshMode(raw: string | undefined, fallback: RefreshMode): RefreshMode {
  const normalized = cleanText(raw || fallback, 24).toLowerCase();
  if (normalized === 'always' || normalized === 'force' || normalized === '1' || normalized === 'true') {
    return 'always';
  }
  if (normalized === 'never' || normalized === 'skip' || normalized === '0' || normalized === 'false') {
    return 'never';
  }
  return 'auto';
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/runtime_proof_release_gate_current.json',
  });
  const profile = parseProfile(readFlag(argv, 'profile'));
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    policyPath: cleanText(
      readFlag(argv, 'policy') || 'tests/tooling/config/release_gates.yaml',
      400,
    ),
    harnessPath: cleanText(
      readFlag(argv, 'harness') || 'core/local/artifacts/runtime_proof_harness_current.json',
      400,
    ),
    adapterChaosPath: cleanText(
      readFlag(argv, 'adapter-chaos') || 'core/local/artifacts/adapter_runtime_chaos_gate_current.json',
      400,
    ),
    metricsOutPath: cleanText(
      readFlag(argv, 'metrics-out') || 'core/local/artifacts/runtime_proof_release_metrics_current.json',
      400,
    ),
    qualityPath: cleanText(
      readFlag(argv, 'quality') || 'artifacts/web_tooling_context_soak_report_latest.json',
      400,
    ),
    tableOutPath: cleanText(
      readFlag(argv, 'table-out') || 'local/workspace/reports/RUNTIME_PROOF_RELEASE_GATE_CURRENT.md',
      400,
    ),
    refreshHarness: parseRefreshMode(readFlag(argv, 'refresh-harness'), 'always'),
    refreshAdapterChaos: parseRefreshMode(readFlag(argv, 'refresh-adapter-chaos'), 'always'),
    profile,
    proofTrack: parseProofTrack(readFlag(argv, 'proof-track')),
  };
}

function parsePolicyYaml(raw: string): ParsedPolicy {
  const policy: ParsedPolicy = {
    version: 1,
    profiles: {},
  };
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
          proof_tracks: {
            synthetic_required: 1,
            empirical_required: 0,
            empirical_min_sample_points: 0,
          },
          memory: { peak_rss_mb_max: 0 },
          queue: { depth_max: 0, depth_p95_max: 0 },
          receipts: { throughput_per_min_min: 0, p95_latency_ms_max: 0 },
          recovery: {
            conduit_recovery_ms_max: 0,
            adapter_restart_count_max: 0,
            adapter_recovery_ms_max: 0,
          },
          stale_surface: { incidents_max: 0 },
          adapter_chaos: {
            baseline_pass_ratio_min: 0,
            fail_closed_ratio_min: 0,
            graduation_ratio_min: 0,
          },
          quality_telemetry: {
            empty_final_max: 0,
            deferred_final_max: 0,
            placeholder_final_max: 0,
            off_topic_final_max: 0,
            meta_status_tool_leak_max: 0,
            web_missing_tool_attempt_max: 0,
            taxonomy_parse_error_max: 0,
          },
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
      if (!Number.isFinite(value)) continue;
      const row = policy.profiles[currentProfile] as any;
      if (row[currentSection] && Object.prototype.hasOwnProperty.call(row[currentSection], key)) {
        row[currentSection][key] = value;
      }
    }
  }
  return policy;
}

function readJson(filePath: string): any {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function readJsonBestEffort(filePath: string): { ok: boolean; payload: any } {
  try {
    return {
      ok: true,
      payload: readJson(filePath),
    };
  } catch (error) {
    return {
      ok: false,
      payload: {
        parse_error: cleanText((error as Error)?.message || 'quality_metrics_unavailable', 220),
      },
    };
  }
}

function parseLastJsonLine(raw: string): any {
  const lines = String(raw || '')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  for (let idx = lines.length - 1; idx >= 0; idx -= 1) {
    try {
      return JSON.parse(lines[idx]);
    } catch {
      // keep scanning
    }
  }
  return null;
}

function shouldRefreshArtifact(
  artifactPath: string,
  mode: RefreshMode,
  revision: string,
): { refresh: boolean; reason: string } {
  if (mode === 'never') {
    return {
      refresh: false,
      reason: 'refresh_disabled',
    };
  }

  if (!fs.existsSync(artifactPath)) {
    return {
      refresh: true,
      reason: 'artifact_missing',
    };
  }

  if (mode === 'always') {
    return {
      refresh: true,
      reason: 'forced_refresh',
    };
  }

  const parsed = readJsonBestEffort(artifactPath);
  if (!parsed.ok) {
    return {
      refresh: true,
      reason: 'artifact_parse_error',
    };
  }

  const artifactRevision = cleanText(parsed.payload?.revision || '', 80);
  if (!artifactRevision || artifactRevision !== revision) {
    return {
      refresh: true,
      reason: 'revision_mismatch',
    };
  }

  return {
    refresh: false,
    reason: 'artifact_fresh',
  };
}

function runSupportScript(root: string, scriptPath: string, args: string[]): { status: number; output: any; detail: string } {
  const entrypoint = path.resolve(root, 'client/runtime/lib/ts_entrypoint.ts');
  const script = path.resolve(root, scriptPath);
  const proc = spawnSync('node', [entrypoint, script, ...args], {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  });
  const output = parseLastJsonLine(String(proc.stdout || ''));
  const status = proc.status ?? 1;
  const detail = cleanText(
    output?.error ||
      proc.error?.message ||
      String(proc.stderr || '').slice(0, 280) ||
      `status=${status}`,
    320,
  );
  return {
    status,
    output,
    detail,
  };
}

function buildCheckLe(id: string, actual: number, threshold: number): GateCheck {
  return {
    id,
    comparator: '<=',
    actual,
    threshold,
    ok: actual <= threshold,
    detail: `${id}: actual=${actual} threshold<=${threshold}`,
  };
}

function buildCheckGe(id: string, actual: number, threshold: number): GateCheck {
  return {
    id,
    comparator: '>=',
    actual,
    threshold,
    ok: actual >= threshold,
    detail: `${id}: actual=${actual} threshold>=${threshold}`,
  };
}

function toMarkdownTable(profile: ProfileId, checks: GateCheck[], metricsPath: string): string {
  const lines = [
    '# Runtime Proof Release Gate',
    '',
    `- profile: ${profile}`,
    `- metrics_json: ${metricsPath}`,
    '',
    '| gate | comparator | actual | threshold | pass |',
    '| --- | --- | ---: | ---: | :---: |',
  ];
  for (const row of checks) {
    lines.push(
      `| ${row.id} | ${row.comparator} | ${row.actual} | ${row.threshold} | ${
        row.ok ? 'yes' : 'no'
      } |`,
    );
  }
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const revision = currentRevision(root);
  if (!args.profile) {
    const payload = {
      ok: false,
      type: 'runtime_proof_release_gate',
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

  const harnessAbsolutePath = path.resolve(root, args.harnessPath);
  const adapterChaosAbsolutePath = path.resolve(root, args.adapterChaosPath);
  const supportRuns: Array<{
    id: string;
    script: string;
    refresh_mode: RefreshMode;
    refreshed: boolean;
    reason: string;
    status: number;
    detail: string;
  }> = [];

  const harnessRefreshPlan = shouldRefreshArtifact(harnessAbsolutePath, args.refreshHarness, revision);
  if (harnessRefreshPlan.refresh) {
    const refresh = runSupportScript(root, 'tests/tooling/scripts/ci/runtime_proof_harness.ts', [
      `--profile=${args.profile}`,
      `--out=${args.harnessPath}`,
      `--proof-track=${args.proofTrack}`,
    ]);
    supportRuns.push({
      id: 'runtime_proof_harness',
      script: 'tests/tooling/scripts/ci/runtime_proof_harness.ts',
      refresh_mode: args.refreshHarness,
      refreshed: true,
      reason: harnessRefreshPlan.reason,
      status: refresh.status,
      detail: refresh.detail,
    });
    if (refresh.status !== 0) {
      const payload = {
        ok: false,
        type: 'runtime_proof_release_gate',
        error: 'runtime_proof_harness_refresh_failed',
        detail: refresh.detail,
        profile: args.profile,
        script: 'tests/tooling/scripts/ci/runtime_proof_harness.ts',
      };
      return emitStructuredResult(payload, {
        outPath: args.outPath,
        strict: args.strict,
        ok: false,
      });
    }
  } else {
    supportRuns.push({
      id: 'runtime_proof_harness',
      script: 'tests/tooling/scripts/ci/runtime_proof_harness.ts',
      refresh_mode: args.refreshHarness,
      refreshed: false,
      reason: harnessRefreshPlan.reason,
      status: 0,
      detail: 'reused_existing_artifact',
    });
  }

  const adapterRefreshPlan = shouldRefreshArtifact(
    adapterChaosAbsolutePath,
    args.refreshAdapterChaos,
    revision,
  );
  if (adapterRefreshPlan.refresh) {
    const refresh = runSupportScript(root, 'tests/tooling/scripts/ci/adapter_runtime_chaos_gate.ts', [
      `--profile=${args.profile}`,
      `--out=${args.adapterChaosPath}`,
    ]);
    supportRuns.push({
      id: 'adapter_runtime_chaos_gate',
      script: 'tests/tooling/scripts/ci/adapter_runtime_chaos_gate.ts',
      refresh_mode: args.refreshAdapterChaos,
      refreshed: true,
      reason: adapterRefreshPlan.reason,
      status: refresh.status,
      detail: refresh.detail,
    });
    if (refresh.status !== 0) {
      const payload = {
        ok: false,
        type: 'runtime_proof_release_gate',
        error: 'adapter_runtime_chaos_refresh_failed',
        detail: refresh.detail,
        profile: args.profile,
        script: 'tests/tooling/scripts/ci/adapter_runtime_chaos_gate.ts',
      };
      return emitStructuredResult(payload, {
        outPath: args.outPath,
        strict: args.strict,
        ok: false,
      });
    }
  } else {
    supportRuns.push({
      id: 'adapter_runtime_chaos_gate',
      script: 'tests/tooling/scripts/ci/adapter_runtime_chaos_gate.ts',
      refresh_mode: args.refreshAdapterChaos,
      refreshed: false,
      reason: adapterRefreshPlan.reason,
      status: 0,
      detail: 'reused_existing_artifact',
    });
  }

  const policyRaw = fs.readFileSync(path.resolve(root, args.policyPath), 'utf8');
  const policy = parsePolicyYaml(policyRaw);
  const profilePolicy = policy.profiles[args.profile];
  if (!profilePolicy) {
    const payload = {
      ok: false,
      type: 'runtime_proof_release_gate',
      error: 'runtime_proof_policy_profile_missing',
      profile: args.profile,
      policy_path: args.policyPath,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  const harness = readJson(harnessAbsolutePath);
  const adapterChaos = readJson(adapterChaosAbsolutePath);
  const qualityRaw = readJsonBestEffort(path.resolve(root, args.qualityPath));
  const metrics = harness?.metrics || {};
  const adapterChaosMetrics = adapterChaos?.metrics || {};
  const adapterRatioInputs = (() => {
    const asFinite = (value: unknown): number | null => {
      const numeric = Number(value);
      return Number.isFinite(numeric) ? numeric : null;
    };
    const baselineMetric = asFinite(adapterChaosMetrics.adapter_baseline_pass_ratio);
    const failClosedMetric = asFinite(adapterChaosMetrics.adapter_chaos_fail_closed_ratio);
    const graduationMetric = asFinite(adapterChaosMetrics.adapter_graduation_ratio);
    if (baselineMetric != null && failClosedMetric != null && graduationMetric != null) {
      return {
        baseline: baselineMetric,
        fail_closed: failClosedMetric,
        graduation: graduationMetric,
        source: 'adapter_runtime_chaos_gate.metrics',
      };
    }
    const manifestRaw = readJsonBestEffort(
      path.resolve(root, 'tests/tooling/config/adapter_graduation_manifest.json'),
    );
    const adapters = Array.isArray(manifestRaw.payload?.adapters)
      ? manifestRaw.payload.adapters
      : [];
    const productionCount = adapters.filter(
      (row: any) => cleanText(row?.tier || '', 40).toLowerCase() === 'production',
    ).length;
    const manifestRatio = adapters.length === 0 ? 0 : productionCount / adapters.length;
    return {
      baseline: baselineMetric ?? manifestRatio,
      fail_closed: failClosedMetric ?? manifestRatio,
      graduation: graduationMetric ?? manifestRatio,
      source:
        adapters.length > 0
          ? 'adapter_graduation_manifest.fallback'
          : 'adapter_ratio_input_unavailable',
    };
  })();
  const proofTracks = harness?.proof_tracks || {};
  const selectedTrack = cleanText(proofTracks?.selected || args.proofTrack, 24);
  const syntheticSamplePoints = Number(proofTracks?.synthetic?.sample_points || 0);
  const empiricalSamplePoints = Number(proofTracks?.empirical?.sample_points || 0);
  const empiricalAvailable = proofTracks?.empirical?.available === true || empiricalSamplePoints > 0;
  const qualityTaxonomy = qualityRaw.payload?.taxonomy || { parse_error: 'taxonomy_missing' };
  const qualityParseError =
    !qualityRaw.ok || cleanText(qualityTaxonomy.parse_error || '', 120).length > 0 ? 1 : 0;
  const qualityMetrics = {
    quality_empty_final: Number(qualityTaxonomy.empty_final || 0),
    quality_deferred_final: Number(qualityTaxonomy.deferred_final || 0),
    quality_placeholder_final: Number(qualityTaxonomy.placeholder_final || 0),
    quality_off_topic_final: Number(qualityTaxonomy.off_topic_final || 0),
    quality_meta_status_tool_leak: Number(qualityTaxonomy.meta_status_tool_leak || 0),
    quality_web_missing_tool_attempt: Number(qualityTaxonomy.web_missing_tool_attempt || 0),
    quality_taxonomy_parse_error: qualityParseError,
  } as Record<string, number>;
  const checks: GateCheck[] = [
    buildCheckGe(
      'proof_track_selected_known',
      ['synthetic', 'empirical', 'dual'].includes(selectedTrack) ? 1 : 0,
      1,
    ),
    buildCheckGe(
      'proof_track_synthetic_sample_points_required',
      syntheticSamplePoints,
      profilePolicy.proof_tracks.synthetic_required > 0 ? 1 : 0,
    ),
    buildCheckGe(
      'proof_track_empirical_required',
      empiricalAvailable ? 1 : 0,
      profilePolicy.proof_tracks.empirical_required,
    ),
    buildCheckGe(
      'proof_track_empirical_sample_points_min',
      empiricalSamplePoints,
      profilePolicy.proof_tracks.empirical_required > 0
        ? profilePolicy.proof_tracks.empirical_min_sample_points
        : 0,
    ),
    buildCheckLe('peak_rss_mb_max', Number(metrics.peak_rss_mb || 0), profilePolicy.memory.peak_rss_mb_max),
    buildCheckLe('queue_depth_max', Number(metrics.queue_depth_max || 0), profilePolicy.queue.depth_max),
    buildCheckLe('queue_depth_p95_max', Number(metrics.queue_depth_p95 || 0), profilePolicy.queue.depth_p95_max),
    buildCheckGe(
      'receipt_throughput_per_min_min',
      Number(metrics.receipt_throughput_per_min || 0),
      profilePolicy.receipts.throughput_per_min_min,
    ),
    buildCheckLe(
      'receipt_p95_latency_ms_max',
      Number(metrics.receipt_p95_latency_ms || 0),
      profilePolicy.receipts.p95_latency_ms_max,
    ),
    buildCheckLe(
      'conduit_recovery_ms_max',
      Number(metrics.conduit_recovery_ms || 0),
      profilePolicy.recovery.conduit_recovery_ms_max,
    ),
    buildCheckLe(
      'adapter_restart_count_max',
      Number(metrics.adapter_restart_count || 0),
      profilePolicy.recovery.adapter_restart_count_max,
    ),
    buildCheckLe(
      'adapter_recovery_ms_max',
      Number(metrics.adapter_recovery_ms || 0),
      profilePolicy.recovery.adapter_recovery_ms_max,
    ),
    buildCheckLe(
      'stale_surface_incidents_max',
      Number(metrics.stale_surface_incidents || 0),
      profilePolicy.stale_surface.incidents_max,
    ),
    buildCheckGe(
      'adapter_baseline_pass_ratio_min',
      Number(adapterRatioInputs.baseline || 0),
      profilePolicy.adapter_chaos.baseline_pass_ratio_min,
    ),
    buildCheckGe(
      'adapter_chaos_fail_closed_ratio_min',
      Number(adapterRatioInputs.fail_closed || 0),
      profilePolicy.adapter_chaos.fail_closed_ratio_min,
    ),
    buildCheckGe(
      'adapter_graduation_ratio_min',
      Number(adapterRatioInputs.graduation || 0),
      profilePolicy.adapter_chaos.graduation_ratio_min,
    ),
    buildCheckLe(
      'quality_empty_final_max',
      Number(qualityMetrics.quality_empty_final || 0),
      profilePolicy.quality_telemetry.empty_final_max,
    ),
    buildCheckLe(
      'quality_deferred_final_max',
      Number(qualityMetrics.quality_deferred_final || 0),
      profilePolicy.quality_telemetry.deferred_final_max,
    ),
    buildCheckLe(
      'quality_placeholder_final_max',
      Number(qualityMetrics.quality_placeholder_final || 0),
      profilePolicy.quality_telemetry.placeholder_final_max,
    ),
    buildCheckLe(
      'quality_off_topic_final_max',
      Number(qualityMetrics.quality_off_topic_final || 0),
      profilePolicy.quality_telemetry.off_topic_final_max,
    ),
    buildCheckLe(
      'quality_meta_status_tool_leak_max',
      Number(qualityMetrics.quality_meta_status_tool_leak || 0),
      profilePolicy.quality_telemetry.meta_status_tool_leak_max,
    ),
    buildCheckLe(
      'quality_web_missing_tool_attempt_max',
      Number(qualityMetrics.quality_web_missing_tool_attempt || 0),
      profilePolicy.quality_telemetry.web_missing_tool_attempt_max,
    ),
    buildCheckLe(
      'quality_taxonomy_parse_error_max',
      Number(qualityMetrics.quality_taxonomy_parse_error || 0),
      profilePolicy.quality_telemetry.taxonomy_parse_error_max,
    ),
  ];

  const failures = checks.filter((row) => !row.ok).map((row) => ({
    id: row.id,
    detail: row.detail,
  }));

  const metricsPayload = {
    ok: failures.length === 0,
    type: 'runtime_proof_release_metrics',
    profile: args.profile,
    policy_version: policy.version,
    checks,
    metrics: {
      ...metrics,
      ...adapterChaosMetrics,
      adapter_ratio_input_source: adapterRatioInputs.source,
      adapter_baseline_pass_ratio_input: Number(adapterRatioInputs.baseline || 0),
      adapter_chaos_fail_closed_ratio_input: Number(adapterRatioInputs.fail_closed || 0),
      adapter_graduation_ratio_input: Number(adapterRatioInputs.graduation || 0),
      ...qualityMetrics,
      proof_track_selected: selectedTrack,
      proof_track_synthetic_sample_points: syntheticSamplePoints,
      proof_track_empirical_sample_points: empiricalSamplePoints,
    },
  };
  writeJsonArtifact(args.metricsOutPath, metricsPayload);
  writeTextArtifact(args.tableOutPath, toMarkdownTable(args.profile, checks, args.metricsOutPath));

  const report = {
    ok: failures.length === 0,
    type: 'runtime_proof_release_gate',
    profile: args.profile,
    generated_at: new Date().toISOString(),
    revision,
    inputs: {
      policy_path: args.policyPath,
      harness_path: args.harnessPath,
      adapter_chaos_path: args.adapterChaosPath,
      quality_path: args.qualityPath,
      proof_track: args.proofTrack,
      metrics_out: args.metricsOutPath,
      table_out: args.tableOutPath,
      refresh_harness: args.refreshHarness,
      refresh_adapter_chaos: args.refreshAdapterChaos,
    },
    support_runs: supportRuns,
    summary: {
      check_count: checks.length,
      failed_count: failures.length,
      pass: failures.length === 0,
      selected_track: selectedTrack,
      empirical_sample_points: empiricalSamplePoints,
    },
    checks,
    failures,
    artifact_paths: [args.metricsOutPath, args.tableOutPath],
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
  parsePolicyYaml,
};
