#!/usr/bin/env tsx

import { execFileSync } from 'node:child_process';
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
    report_not_ok_max: number;
    report_read_error_max: number;
    denied_actions_max: number;
    pause_reason_events_max: number;
    merkle_chain_continuity_failures_max: number;
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

type SupportRun = {
  id: string;
  script: string;
  refresh_mode: RefreshMode;
  refreshed: boolean;
  reason: string;
  status: number;
  detail: string;
};

const EMPIRICAL_REQUIRED_SOURCE_IDS_BY_PROFILE: Record<ProfileId, string[]> = {
  rich: ['runtime_boundedness_inspect', 'ops_ipc_bridge_stability_soak', 'dashboard_snapshot'],
  pure: ['runtime_boundedness_inspect', 'ops_ipc_bridge_stability_soak', 'support_bundle'],
  'tiny-max': ['runtime_boundedness_inspect', 'ops_ipc_bridge_stability_soak', 'support_bundle'],
};

const EMPIRICAL_REQUIRED_METRIC_KEYS_BY_PROFILE: Record<ProfileId, string[]> = {
  rich: [
    'peak_rss_mb',
    'queue_depth_max',
    'queue_depth_p95',
    'receipt_throughput_per_min',
    'receipt_p95_latency_ms',
    'conduit_recovery_ms',
  ],
  pure: ['peak_rss_mb', 'receipt_throughput_per_min', 'receipt_p95_latency_ms', 'conduit_recovery_ms'],
  'tiny-max': ['peak_rss_mb', 'receipt_throughput_per_min', 'receipt_p95_latency_ms', 'conduit_recovery_ms'],
};

type QualityRefreshTarget = {
  id: string;
  script: string;
};

function resolveQualityRefreshTarget(relPath: string): QualityRefreshTarget | null {
  const normalized = cleanText(relPath || '', 500).replace(/\\/g, '/');
  if (
    normalized === 'artifacts/web_tooling_context_soak_report_latest.json' ||
    normalized.endsWith('/web_tooling_context_soak_report_latest.json')
  ) {
    return {
      id: 'web_tooling_context_soak',
      script: 'tests/tooling/scripts/ci/web_tooling_context_soak.ts',
    };
  }
  if (
    normalized === 'artifacts/workspace_tooling_context_soak_report_latest.json' ||
    normalized.endsWith('/workspace_tooling_context_soak_report_latest.json')
  ) {
    return {
      id: 'workspace_tooling_context_soak',
      script: 'tests/tooling/scripts/ci/workspace_tooling_context_soak.ts',
    };
  }
  return null;
}

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

function parseQualityPaths(raw: string | undefined): string[] {
  const input = cleanText(raw || '', 800);
  const parts = input
    .split(',')
    .map((row) => cleanText(row || '', 400))
    .filter(Boolean);
  const fallback = [
    'artifacts/web_tooling_context_soak_report_latest.json',
    'artifacts/workspace_tooling_context_soak_report_latest.json',
  ];
  const selected = parts.length > 0 ? parts : fallback;
  const deduped: string[] = [];
  for (const row of selected) {
    if (!deduped.includes(row)) deduped.push(row);
  }
  return deduped;
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/runtime_proof_release_gate_current.json',
  });
  const profile = parseProfile(readFlag(argv, 'profile'));
  const qualityPaths = parseQualityPaths(
    readFlag(argv, 'quality-paths') || readFlag(argv, 'quality'),
  );
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
    qualityPaths,
    runtimeLaneStatePath: cleanText(
      readFlag(argv, 'runtime-lane-state') ||
        'local/state/infring_agent_surface/runtime_lane_state.json',
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
            report_not_ok_max: 0,
            report_read_error_max: 0,
            denied_actions_max: 0,
            pause_reason_events_max: 0,
            merkle_chain_continuity_failures_max: 0,
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
  try {
    const stdout = execFileSync('node', [entrypoint, script, ...args], {
      cwd: root,
      encoding: 'utf8',
      maxBuffer: 64 * 1024 * 1024,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const output = parseLastJsonLine(String(stdout || ''));
    return {
      status: 0,
      output,
      detail: cleanText(output?.error || 'ok', 320),
    };
  } catch (error) {
    const err = error as {
      status?: number;
      stdout?: string | Buffer;
      stderr?: string | Buffer;
      message?: string;
    };
    const stdout = String(err.stdout || '');
    const stderr = String(err.stderr || '');
    const output = parseLastJsonLine(stdout);
    const status = Number.isFinite(err.status) ? Number(err.status) : 1;
    const detail = cleanText(
      output?.error || err.message || stderr.slice(0, 280) || `status=${status}`,
      320,
    );
    return {
      status,
      output,
      detail,
    };
  }
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

function buildRequiredCompletenessCheck(id: string, required: boolean, missing: string[]): GateCheck {
  const actual = missing.length === 0 ? 1 : 0;
  const threshold = required ? 1 : 0;
  const missingDetail = missing.length > 0 ? ` missing=${missing.join(',')}` : '';
  return {
    id,
    comparator: '>=',
    actual,
    threshold,
    ok: actual >= threshold,
    detail: `${id}: actual=${actual} threshold>=${threshold}${missingDetail}`,
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
  if (args.strict && (args.refreshHarness === 'never' || args.refreshAdapterChaos === 'never')) {
    const payload = {
      ok: false,
      type: 'runtime_proof_release_gate',
      error: 'runtime_proof_refresh_mode_never_forbidden',
      detail: 'strict mode requires refresh-harness and refresh-adapter-chaos to be auto or always',
      profile: args.profile,
      refresh_harness: args.refreshHarness,
      refresh_adapter_chaos: args.refreshAdapterChaos,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }
  const supportRuns: SupportRun[] = [];

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
  const reusedSupportRuns = supportRuns.filter((row) => !row.refreshed);
  if (args.strict && reusedSupportRuns.length > 0) {
    const payload = {
      ok: false,
      type: 'runtime_proof_release_gate',
      error: 'runtime_proof_support_artifact_reused',
      detail: 'strict mode requires fresh support artifacts generated in this invocation',
      profile: args.profile,
      reused_support_runs: reusedSupportRuns.map((row) => ({
        id: row.id,
        script: row.script,
        refresh_mode: row.refresh_mode,
        reason: row.reason,
        detail: row.detail,
      })),
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  for (const relPath of args.qualityPaths) {
    const absolute = path.resolve(root, relPath);
    if (fs.existsSync(absolute)) continue;
    const refreshTarget = resolveQualityRefreshTarget(relPath);
    if (!refreshTarget) continue;
    const refresh = runSupportScript(root, refreshTarget.script, []);
    supportRuns.push({
      id: refreshTarget.id,
      script: refreshTarget.script,
      refresh_mode: 'auto',
      refreshed: true,
      reason: 'artifact_missing',
      status: refresh.status,
      detail: refresh.detail,
    });
    if (refresh.status !== 0 || !fs.existsSync(absolute)) {
      const payload = {
        ok: false,
        type: 'runtime_proof_release_gate',
        error: 'quality_telemetry_refresh_failed',
        detail: cleanText(
          refresh.detail || 'quality telemetry artifact was missing and refresh did not produce it',
          320,
        ),
        profile: args.profile,
        quality_artifact_path: relPath,
        script: refreshTarget.script,
      };
      return emitStructuredResult(payload, {
        outPath: args.outPath,
        strict: args.strict,
        ok: false,
      });
    }
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
  const qualityReports = args.qualityPaths.map((relPath) => {
    const parsed = readJsonBestEffort(path.resolve(root, relPath));
    return {
      path: relPath,
      ok: parsed.ok,
      payload: parsed.payload,
    };
  });
  const runtimeLaneStateRaw = readJsonBestEffort(path.resolve(root, args.runtimeLaneStatePath));
  const metrics = harness?.metrics || {};
  const adapterChaosMetrics = adapterChaos?.metrics || {};
  if (adapterChaos?.ok === false) {
    const payload = {
      ok: false,
      type: 'runtime_proof_release_gate',
      error: 'adapter_runtime_chaos_gate_not_ok',
      detail: cleanText(
        String(
          adapterChaos?.error || adapterChaos?.summary?.error || 'adapter_runtime_chaos_gate_reported_not_ok',
        ),
        320,
      ),
      profile: args.profile,
      adapter_chaos_path: args.adapterChaosPath,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }
  const adapterRatioInputs = (() => {
    const asFinite = (value: unknown): number | null => {
      const numeric = Number(value);
      return Number.isFinite(numeric) ? numeric : null;
    };
    const baseline = asFinite(adapterChaosMetrics.adapter_baseline_pass_ratio);
    const failClosed = asFinite(adapterChaosMetrics.adapter_chaos_fail_closed_ratio);
    const graduation = asFinite(adapterChaosMetrics.adapter_graduation_ratio);
    const missing_fields = [
      ...(baseline == null ? ['adapter_baseline_pass_ratio'] : []),
      ...(failClosed == null ? ['adapter_chaos_fail_closed_ratio'] : []),
      ...(graduation == null ? ['adapter_graduation_ratio'] : []),
    ];
    return {
      ok: missing_fields.length === 0,
      baseline: baseline ?? 0,
      fail_closed: failClosed ?? 0,
      graduation: graduation ?? 0,
      source: 'adapter_runtime_chaos_gate.metrics',
      missing_fields,
    };
  })();
  if (!adapterRatioInputs.ok) {
    const payload = {
      ok: false,
      type: 'runtime_proof_release_gate',
      error: 'adapter_runtime_chaos_ratio_inputs_missing',
      detail:
        'adapter ratio metrics must be emitted by adapter_runtime_chaos_gate; manifest fallback is disabled',
      profile: args.profile,
      adapter_chaos_path: args.adapterChaosPath,
      adapter_ratio_input_source: adapterRatioInputs.source,
      missing_fields: adapterRatioInputs.missing_fields,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }
  const proofTracks = harness?.proof_tracks || {};
  const empiricalTrack = proofTracks?.empirical || {};
  const empiricalRequired = profilePolicy.proof_tracks.empirical_required > 0;
  const empiricalProvidedKeys = new Set(
    (Array.isArray(empiricalTrack?.provided_keys) ? empiricalTrack.provided_keys : [])
      .map((key: unknown) => cleanText(String(key || ''), 80))
      .filter(Boolean),
  );
  const empiricalSources = Array.isArray(empiricalTrack?.sources) ? empiricalTrack.sources : [];
  const loadedEmpiricalSourceIds = new Set(
    empiricalSources
      .filter((row: any) => row?.loaded === true && Number(row?.sample_points || 0) > 0)
      .map((row: any) => cleanText(String(row?.id || ''), 80))
      .filter(Boolean),
  );
  const requiredEmpiricalSourceIds = EMPIRICAL_REQUIRED_SOURCE_IDS_BY_PROFILE[args.profile];
  const missingEmpiricalSourceIds = requiredEmpiricalSourceIds.filter(
    (id) => !loadedEmpiricalSourceIds.has(id),
  );
  const requiredEmpiricalProvidedKeys = EMPIRICAL_REQUIRED_METRIC_KEYS_BY_PROFILE[args.profile];
  const missingEmpiricalProvidedKeys = requiredEmpiricalProvidedKeys.filter(
    (key) => !empiricalProvidedKeys.has(key),
  );
  const requiredEmpiricalPositiveMetricKeys = requiredEmpiricalProvidedKeys;
  const nonPositiveEmpiricalMetricKeys = requiredEmpiricalPositiveMetricKeys.filter(
    (key) => Number(empiricalTrack?.metrics?.[key] || 0) <= 0,
  );
  const selectedTrack = cleanText(proofTracks?.selected || args.proofTrack, 24);
  const syntheticSamplePoints = Number(proofTracks?.synthetic?.sample_points || 0);
  const empiricalSamplePoints = Number(empiricalTrack?.sample_points || 0);
  const empiricalAvailable = empiricalTrack?.available === true || empiricalSamplePoints > 0;
  const qualityTaxonomyRows = qualityReports.map((row) => {
    const taxonomy = row.payload?.taxonomy && typeof row.payload.taxonomy === 'object'
      ? row.payload.taxonomy
      : {};
    return {
      path: row.path,
      ok: row.ok,
      payload_ok: row.payload?.ok,
      taxonomy,
      parse_error: cleanText(taxonomy?.parse_error || '', 120),
    };
  });
  const qualityTaxonomy = qualityTaxonomyRows.reduce<Record<string, number>>((acc, row) => {
    const asNumber = (value: unknown): number => {
      const numeric = Number(value);
      return Number.isFinite(numeric) ? numeric : 0;
    };
    acc.empty_final += asNumber(row.taxonomy?.empty_final);
    acc.deferred_final += asNumber(row.taxonomy?.deferred_final);
    acc.placeholder_final += asNumber(row.taxonomy?.placeholder_final);
    acc.off_topic_final += asNumber(row.taxonomy?.off_topic_final);
    acc.meta_status_tool_leak += asNumber(row.taxonomy?.meta_status_tool_leak);
    acc.web_missing_tool_attempt += asNumber(row.taxonomy?.web_missing_tool_attempt);
    return acc;
  }, {
    empty_final: 0,
    deferred_final: 0,
    placeholder_final: 0,
    off_topic_final: 0,
    meta_status_tool_leak: 0,
    web_missing_tool_attempt: 0,
  });
  const qualityParseErrorRows = qualityTaxonomyRows.filter((row) => {
    if (!row.ok) return false;
    if (row.payload_ok === false) return false;
    return row.parse_error.length > 0;
  });
  const qualityParseError = qualityParseErrorRows.length;
  const qualityReportNotOkCount = qualityTaxonomyRows.filter((row) => row.payload_ok === false).length;
  const qualityReportReadErrorCount = qualityTaxonomyRows.filter((row) => !row.ok).length;
  const runtimeLaneCounters = runtimeLaneStateRaw.payload?.release_gate_counters || {};
  const runtimeLanePauseCounts = runtimeLaneCounters.pause_reason_counts || {};
  const runtimeLanePauseReasonsTotal = Number(
    runtimeLaneCounters.pause_reasons_total ||
      Object.values(runtimeLanePauseCounts).reduce((sum: number, value: any) => {
        const numeric = Number(value);
        if (!Number.isFinite(numeric)) return sum;
        return sum + numeric;
      }, 0),
  );
  const qualityMetrics = {
    quality_empty_final: Number(qualityTaxonomy.empty_final || 0),
    quality_deferred_final: Number(qualityTaxonomy.deferred_final || 0),
    quality_placeholder_final: Number(qualityTaxonomy.placeholder_final || 0),
    quality_off_topic_final: Number(qualityTaxonomy.off_topic_final || 0),
    quality_meta_status_tool_leak: Number(qualityTaxonomy.meta_status_tool_leak || 0),
    quality_web_missing_tool_attempt: Number(qualityTaxonomy.web_missing_tool_attempt || 0),
    quality_taxonomy_parse_error: qualityParseError,
    quality_report_not_ok_count: qualityReportNotOkCount,
    quality_report_read_error_count: qualityReportReadErrorCount,
    quality_reports_total: qualityTaxonomyRows.length,
    quality_denied_actions: Number(runtimeLaneCounters.denied_actions_total || 0),
    quality_pause_reason_events: Number(runtimeLanePauseReasonsTotal || 0),
    quality_merkle_chain_continuity_failures: Number(
      runtimeLaneCounters.merkle_chain_continuity_failures_total || 0,
    ),
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
      'proof_track_empirical_selected_when_required',
      selectedTrack === 'empirical' || selectedTrack === 'dual' ? 1 : 0,
      empiricalRequired ? 1 : 0,
    ),
    buildCheckGe(
      'proof_track_empirical_sample_points_min',
      empiricalSamplePoints,
      empiricalRequired ? profilePolicy.proof_tracks.empirical_min_sample_points : 0,
    ),
    buildRequiredCompletenessCheck(
      'proof_track_empirical_required_sources_loaded',
      empiricalRequired,
      missingEmpiricalSourceIds,
    ),
    buildRequiredCompletenessCheck(
      'proof_track_empirical_required_metrics_present',
      empiricalRequired,
      missingEmpiricalProvidedKeys,
    ),
    buildRequiredCompletenessCheck(
      'proof_track_empirical_required_positive_metrics',
      empiricalRequired,
      nonPositiveEmpiricalMetricKeys,
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
    buildCheckLe(
      'quality_report_not_ok_max',
      Number(qualityMetrics.quality_report_not_ok_count || 0),
      profilePolicy.quality_telemetry.report_not_ok_max,
    ),
    buildCheckLe(
      'quality_report_read_error_max',
      Number(qualityMetrics.quality_report_read_error_count || 0),
      profilePolicy.quality_telemetry.report_read_error_max,
    ),
    buildCheckLe(
      'quality_denied_actions_max',
      Number(qualityMetrics.quality_denied_actions || 0),
      profilePolicy.quality_telemetry.denied_actions_max,
    ),
    buildCheckLe(
      'quality_pause_reason_events_max',
      Number(qualityMetrics.quality_pause_reason_events || 0),
      profilePolicy.quality_telemetry.pause_reason_events_max,
    ),
    buildCheckLe(
      'quality_merkle_chain_continuity_failures_max',
      Number(qualityMetrics.quality_merkle_chain_continuity_failures || 0),
      profilePolicy.quality_telemetry.merkle_chain_continuity_failures_max,
    ),
  ];

  const failures = checks.filter((row) => !row.ok).map((row) => ({
    id: row.id,
    detail: row.detail,
  }));
  const profileRequirements = {
    synthetic_required: Number(profilePolicy.proof_tracks.synthetic_required || 0),
    empirical_required: Number(profilePolicy.proof_tracks.empirical_required || 0),
    empirical_min_sample_points: Number(
      profilePolicy.proof_tracks.empirical_min_sample_points || 0,
    ),
  };

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
      proof_track_empirical_required_sources_missing: missingEmpiricalSourceIds,
      proof_track_empirical_required_metrics_missing: missingEmpiricalProvidedKeys,
      proof_track_empirical_required_positive_metrics_missing: nonPositiveEmpiricalMetricKeys,
      quality_input_paths: args.qualityPaths,
      quality_report_statuses: qualityTaxonomyRows.map((row) => ({
        path: row.path,
        read_ok: row.ok,
        payload_ok: row.payload_ok === true ? true : row.payload_ok === false ? false : null,
        parse_error: row.parse_error,
      })),
      quality_taxonomy_parse_error_rows: qualityParseErrorRows.map((row) => ({
        path: row.path,
        parse_error: row.parse_error,
      })),
      runtime_lane_state_path: args.runtimeLaneStatePath,
    },
    profile_requirements: profileRequirements,
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
      quality_paths: args.qualityPaths,
      runtime_lane_state_path: args.runtimeLaneStatePath,
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
      support_runs_total: supportRuns.length,
      support_runs_refreshed: supportRuns.filter((row) => row.refreshed).length,
      support_runs_reused: supportRuns.filter((row) => !row.refreshed).length,
    },
    profile_requirements: profileRequirements,
    effective_policy: profileRequirements,
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
