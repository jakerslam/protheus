#!/usr/bin/env node
/* eslint-disable no-console */
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import {
  CANONICAL_THROUGHPUT_METRIC,
  collectBenchmarkAliasLeaks,
  collectBenchmarkPathLeaks,
  renderBenchmarkSnapshotBlock,
  sanitizePublicBenchmarkReport,
  upsertBenchmarkSnapshotBlock,
} from './benchmark_public_surface';

const ROOT = resolve(__dirname, '..', '..', '..', '..');
const DEFAULT_REPORT_PATH = 'docs/client/reports/benchmark_matrix_run_latest.json';
const LEGACY_REPORT_PATH = 'docs/client/reports/benchmark_matrix_run_2026-03-06.json';
const LEGACY_FULL_INSTALL_REPORT_PATH = 'docs/client/reports/benchmark_matrix_run_2026-03-06_full_install.json';

type Options = {
  retries: number;
  retryDelayMs: number;
  reportPath: string;
  mirrorLegacy: boolean;
  cargoProfile: 'debug' | 'release' | 'release-speed';
  throughputUncached: boolean;
  refreshRuntime: boolean;
  preflightMaxLoadPerCore: number;
  preflightMaxNoiseCvPct: number;
  preflightNoiseSampleMs: number;
  preflightNoiseRounds: number;
};

function parseCargoProfile(raw: string | null | undefined): 'debug' | 'release' | 'release-speed' {
  const normalized = String(raw || '')
    .trim()
    .toLowerCase();
  if (normalized === 'debug') return 'debug';
  if (normalized === 'release') return 'release';
  if (normalized === 'release-speed') return 'release-speed';
  return 'release-speed';
}

function parseBool(raw: string | null | undefined, fallback: boolean): boolean {
  if (raw == null) return fallback;
  const v = String(raw).trim().toLowerCase();
  if (!v) return fallback;
  if (['1', 'true', 'yes', 'on'].includes(v)) return true;
  if (['0', 'false', 'no', 'off'].includes(v)) return false;
  return fallback;
}

function parseNumber(raw: string | null | undefined, fallback: number, min: number, max: number): number {
  const n = Number(raw);
  if (!Number.isFinite(n)) return fallback;
  return Math.min(max, Math.max(min, n));
}

function parseArgs(argv: string[]): Options {
  const opts: Options = {
    retries: 8,
    retryDelayMs: 15_000,
    reportPath: DEFAULT_REPORT_PATH,
    mirrorLegacy: true,
    cargoProfile: parseCargoProfile(process.env.INFRING_BENCH_CARGO_PROFILE),
    throughputUncached: false,
    refreshRuntime: true,
    preflightMaxLoadPerCore: 0.9,
    preflightMaxNoiseCvPct: 12.5,
    preflightNoiseSampleMs: 600,
    preflightNoiseRounds: 5
  };
  for (const arg of argv) {
    const token = String(arg || '').trim();
    if (!token.startsWith('--')) continue;
    const [key, value = ''] = token.slice(2).split('=', 2);
    if (key === 'retries') opts.retries = parseNumber(value, opts.retries, 1, 40);
    if (key === 'retry-delay-ms') opts.retryDelayMs = parseNumber(value, opts.retryDelayMs, 1000, 180_000);
    if (key === 'report-path') opts.reportPath = value.trim() || opts.reportPath;
    if (key === 'mirror-legacy') opts.mirrorLegacy = parseBool(value, opts.mirrorLegacy);
    if (key === 'release') {
      opts.cargoProfile = parseBool(value, true) ? 'release' : 'debug';
    }
    if (key === 'cargo-profile') {
      opts.cargoProfile = parseCargoProfile(value);
    }
    if (key === 'throughput-uncached') opts.throughputUncached = parseBool(value, opts.throughputUncached);
    if (key === 'refresh-runtime') opts.refreshRuntime = parseBool(value, opts.refreshRuntime);
    if (key === 'preflight-max-load-per-core') {
      opts.preflightMaxLoadPerCore = parseNumber(value, opts.preflightMaxLoadPerCore, 0.01, 8);
    }
    if (key === 'preflight-max-noise-cv-pct') {
      opts.preflightMaxNoiseCvPct = parseNumber(value, opts.preflightMaxNoiseCvPct, 0.01, 100);
    }
    if (key === 'preflight-noise-sample-ms') {
      opts.preflightNoiseSampleMs = parseNumber(value, opts.preflightNoiseSampleMs, 100, 5000);
    }
    if (key === 'preflight-noise-rounds') {
      opts.preflightNoiseRounds = parseNumber(value, opts.preflightNoiseRounds, 1, 20);
    }
  }
  return opts;
}

function sleep(ms: number): void {
  const timeout = Math.max(0, Math.floor(ms));
  if (!timeout) return;
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, timeout);
}

function run(cmd: string, args: string[]) {
  return spawnSync(cmd, args, {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: 'pipe',
    env: { ...process.env, INFRING_ROOT: ROOT, PROTHEUS_ROOT: ROOT }
  });
}

function parseJsonStdout(stdout: string): any {
  const text = String(stdout || '').trim();
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

function asFiniteNumber(value: unknown): number | null {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

type E2EThroughputMeasurement = {
  value: number | null;
  source: string;
  samples: number;
  sampleTotalMs: number;
};

function measureRichEndToEndCommandPathOpsPerSec(): E2EThroughputMeasurement {
  const samples = parseNumber(process.env.INFRING_BENCH_E2E_SAMPLES, 5, 2, 30);
  const command = process.execPath || 'node';
  const args = [
    'client/runtime/lib/ts_entrypoint.ts',
    'client/runtime/systems/ops/security_layer_inventory_gate.ts',
    'status',
    '--strict=1',
    '--json=1',
  ];
  const env = {
    ...process.env,
    INFRING_ROOT: ROOT,
    PROTHEUS_ROOT: ROOT,
    INFRING_OPS_IPC_DAEMON: '1',
    INFRING_OPS_IPC_STRICT: '1',
    INFRING_OPS_USE_PREBUILT: '1',
    PROTHEUS_OPS_USE_PREBUILT: '1',
    INFRING_OPS_LOCAL_TIMEOUT_MS: '60000',
  };
  const warm = spawnSync(command, args, {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: 'pipe',
    env,
    timeout: 120_000,
  });
  if ((warm.status ?? 1) !== 0) {
    return {
      value: null,
      source: 'measurement_failed:warmup_failed',
      samples,
      sampleTotalMs: 0,
    };
  }
  const started = process.hrtime.bigint();
  for (let idx = 0; idx < samples; idx += 1) {
    const proc = spawnSync(command, args, {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: 'pipe',
      env,
      timeout: 120_000,
    });
    if ((proc.status ?? 1) !== 0) {
      return {
        value: null,
        source: 'measurement_failed:sample_failed',
        samples,
        sampleTotalMs: Number(process.hrtime.bigint() - started) / 1_000_000,
      };
    }
  }
  const elapsedMs = Number(process.hrtime.bigint() - started) / 1_000_000;
  if (!Number.isFinite(elapsedMs) || elapsedMs <= 0) {
    return {
      value: null,
      source: 'measurement_failed:elapsed_invalid',
      samples,
      sampleTotalMs: 0,
    };
  }
  const opsPerSec = Number(((samples * 1000) / elapsedMs).toFixed(2));
  return {
    value: Number.isFinite(opsPerSec) ? opsPerSec : null,
    source: 'measured:security_layer_inventory_gate_status_bridge',
    samples,
    sampleTotalMs: elapsedMs,
  };
}

function assignKernelSharedThroughputMetric(project: any): void {
  if (!project || typeof project !== 'object') return;
  const legacy = asFiniteNumber((project as any).tasks_per_sec);
  if (legacy != null) {
    (project as any).kernel_shared_workload_ops_per_sec = legacy;
  }
}

function assertRunOk(label: string, cmd: string, args: string[]): void {
  const proc = run(cmd, args);
  if ((proc.status ?? 1) !== 0) {
    const stderr = String(proc.stderr || '').trim();
    const stdout = String(proc.stdout || '').trim();
    throw new Error(
      `${label}_failed: ${cmd} ${args.join(' ')}\nstdout=${stdout}\nstderr=${stderr}`
    );
  }
}

function buildArgs(
  manifestPath: string,
  bin: string,
  cargoProfile: 'debug' | 'release' | 'release-speed'
): string[] {
  const args = ['build', '--quiet', '--manifest-path', manifestPath, '--bin', bin];
  if (cargoProfile === 'release') {
    args.splice(2, 0, '--release');
  } else if (cargoProfile !== 'debug') {
    args.splice(2, 0, '--profile', cargoProfile);
  }
  return args;
}

function readText(path: string): string {
  return readFileSync(resolve(ROOT, path), 'utf8');
}

function writeText(path: string, body: string): void {
  ensureParent(path);
  writeFileSync(resolve(ROOT, path), body, 'utf8');
}

function benchmarkArgs(mode: 'status' | 'run', options: Options): string[] {
  return [
    'benchmark-matrix',
    mode,
    `--refresh-runtime=${mode === 'run' && options.refreshRuntime ? 1 : 0}`,
    `--throughput-uncached=${options.throughputUncached ? 1 : 0}`,
    '--benchmark-preflight=1',
    `--preflight-max-load-per-core=${options.preflightMaxLoadPerCore}`,
    `--preflight-max-noise-cv-pct=${options.preflightMaxNoiseCvPct}`,
    `--preflight-noise-sample-ms=${options.preflightNoiseSampleMs}`,
    `--preflight-noise-rounds=${options.preflightNoiseRounds}`
  ];
}

function ensureParent(path: string): void {
  mkdirSync(dirname(resolve(ROOT, path)), { recursive: true });
}

function writeJson(path: string, payload: any): void {
  ensureParent(path);
  writeFileSync(resolve(ROOT, path), `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function main(): void {
  const options = parseArgs(process.argv.slice(2));
  const profileDir = options.cargoProfile;
  const infringOpsBin = `target/${profileDir}/infring-ops`;

  assertRunOk(
    `build_infring_ops_${options.cargoProfile}`,
    'cargo',
    buildArgs('core/layer0/ops/Cargo.toml', 'infring-ops', options.cargoProfile)
  );
  assertRunOk(
    `build_protheus_ops_compat_${options.cargoProfile}`,
    'cargo',
    buildArgs('core/layer0/ops/Cargo.toml', 'protheus-ops', options.cargoProfile)
  );
  assertRunOk(
    `build_infringd_${options.cargoProfile}`,
    'cargo',
    buildArgs('core/layer0/ops/Cargo.toml', 'infringd', options.cargoProfile)
  );
  assertRunOk(
    `build_protheusd_compat_${options.cargoProfile}`,
    'cargo',
    buildArgs('core/layer0/ops/Cargo.toml', 'protheusd', options.cargoProfile)
  );
  assertRunOk(
    `build_pure_workspace_${options.cargoProfile}`,
    'cargo',
    buildArgs('client/pure-workspace/Cargo.toml', 'infring-pure-workspace', options.cargoProfile)
  );
  assertRunOk(
    `build_protheus_pure_workspace_compat_${options.cargoProfile}`,
    'cargo',
    buildArgs(
      'client/pure-workspace/Cargo.toml',
      'protheus-pure-workspace',
      options.cargoProfile
    )
  );

  const attempts: Array<{ attempt: number; ok: boolean; blockers: string[] }> = [];
  let preflightOk = false;
  for (let attempt = 1; attempt <= options.retries; attempt += 1) {
    const proc = run(infringOpsBin, benchmarkArgs('status', options));
    const payload = parseJsonStdout(proc.stdout) || {};
    const preflight = payload?.benchmark_preflight || {};
    const blockers = Array.isArray(preflight.blockers)
      ? preflight.blockers.map((v: unknown) => String(v))
      : [];
    const ok = (proc.status ?? 1) === 0 && payload?.ok === true;
    attempts.push({ attempt, ok, blockers });
    if (ok) {
      preflightOk = true;
      break;
    }
    const isPreflightFail =
      String(payload?.error || '').startsWith('benchmark_preflight_failed:') ||
      blockers.length > 0;
    if (!isPreflightFail || attempt >= options.retries) break;
    sleep(options.retryDelayMs);
  }
  if (!preflightOk) {
    throw new Error(`benchmark_preflight_never_passed:${JSON.stringify(attempts)}`);
  }

  const runProc = run(infringOpsBin, benchmarkArgs('run', options));
  if ((runProc.status ?? 1) !== 0) {
    throw new Error(
      `benchmark_matrix_run_failed:status=${runProc.status}\nstdout=${String(runProc.stdout || '').trim()}\nstderr=${String(runProc.stderr || '').trim()}`
    );
  }
  const report = parseJsonStdout(runProc.stdout);
  if (!report || report.ok !== true) {
    throw new Error('benchmark_matrix_run_invalid_json');
  }
  const sanitizedReport = sanitizePublicBenchmarkReport(report, ROOT);
  const unsanitizedPaths = collectBenchmarkPathLeaks(sanitizedReport);
  if (unsanitizedPaths.length > 0) {
    throw new Error(`benchmark_public_report_contains_absolute_paths:${JSON.stringify(unsanitizedPaths)}`);
  }
  const legacyAliasLeaks = collectBenchmarkAliasLeaks(sanitizedReport);
  if (legacyAliasLeaks.length > 0) {
    throw new Error(
      `benchmark_public_report_contains_legacy_aliases:${JSON.stringify(legacyAliasLeaks)}`
    );
  }

  sanitizedReport.benchmark_refresh_context = {
    build_profile: profileDir,
    refresh_script: 'tests/tooling/scripts/ci/benchmark_matrix_refresh.ts',
    throughput_uncached: options.throughputUncached,
    refresh_runtime: options.refreshRuntime
  };

  const projects =
    sanitizedReport.projects && typeof sanitizedReport.projects === 'object'
      ? sanitizedReport.projects
      : {};
  const rich = projects['InfRing (rich)'] || projects.Infring || sanitizedReport.infring_measured;
  const pure = projects['InfRing (pure)'] || sanitizedReport.pure_workspace_measured;
  const tiny = projects['InfRing (tiny-max)'] || sanitizedReport.pure_workspace_tiny_max_measured;
  if (!rich || !pure || !tiny) {
    throw new Error(
      `benchmark_matrix_modes_missing:rich=${!!rich},pure=${!!pure},tiny_max=${!!tiny}`
    );
  }
  const richEngineStartMs = Number((rich as any).engine_start_ms);
  const richGatewaySupervisorOrchestrationMs = Number(
    (rich as any).gateway_supervisor_orchestration_ms
  );
  const richKernelReadyMs = Number((rich as any).kernel_ready_ms);
  const richGatewayReadyMs = Number((rich as any).gateway_ready_ms);
  const richDashboardInteractiveMs = Number((rich as any).dashboard_interactive_ms);
  if (
    !Number.isFinite(richEngineStartMs) ||
    !Number.isFinite(richGatewaySupervisorOrchestrationMs) ||
    !Number.isFinite(richKernelReadyMs) ||
    !Number.isFinite(richGatewayReadyMs) ||
    !Number.isFinite(richDashboardInteractiveMs)
  ) {
    throw new Error(
      `benchmark_matrix_rich_startup_breakdown_missing:engine_start_ms=${String(
        (rich as any).engine_start_ms
      )},gateway_supervisor_orchestration_ms=${String(
        (rich as any).gateway_supervisor_orchestration_ms
      )},kernel_ready_ms=${String(
        (rich as any).kernel_ready_ms
      )},gateway_ready_ms=${String(
        (rich as any).gateway_ready_ms
      )},dashboard_interactive_ms=${String(
        (rich as any).dashboard_interactive_ms
      )}`
    );
  }
  assignKernelSharedThroughputMetric(rich);
  assignKernelSharedThroughputMetric(pure);
  assignKernelSharedThroughputMetric(tiny);
  const sharedFamily = {
    metric: CANONICAL_THROUGHPUT_METRIC,
    measurement_scope: 'shared_kernel_workload_baseline',
    end_to_end: false,
  };
  const readinessFamily = {
    metric: 'cold_start_ms',
    measurement_scope: 'status_path_readiness',
    end_to_end: false,
    source: 'benchmark_matrix_status_mode',
  };
  const e2eMeasurement = measureRichEndToEndCommandPathOpsPerSec();
  const richEndToEndOpsPerSec = e2eMeasurement.value;
  if (richEndToEndOpsPerSec != null) {
    (rich as any).rich_end_to_end_command_path_ops_per_sec = richEndToEndOpsPerSec;
  } else {
    (rich as any).rich_end_to_end_command_path_ops_per_sec = null;
  }
  (rich as any).rich_end_to_end_command_path_source = e2eMeasurement.source;
  (rich as any).rich_end_to_end_command_path_samples = e2eMeasurement.samples;
  (rich as any).rich_end_to_end_command_path_sample_total_ms = Number(
    e2eMeasurement.sampleTotalMs.toFixed(3)
  );
  (rich as any).benchmark_metric_families = {
    kernel_shared_workload: { ...sharedFamily, source: String((rich as any).throughput_source || '').trim() || null },
    rich_end_to_end_command_path: {
      metric: 'rich_end_to_end_command_path_ops_per_sec',
      measurement_scope: 'command_path_status_bridge',
      end_to_end: true,
      source: e2eMeasurement.source,
      samples: e2eMeasurement.samples,
    },
    rich_status_path_readiness: readinessFamily,
  };
  (pure as any).benchmark_metric_families = {
    kernel_shared_workload: { ...sharedFamily, source: String((pure as any).throughput_source || '').trim() || null },
    rich_status_path_readiness: readinessFamily,
  };
  (tiny as any).benchmark_metric_families = {
    kernel_shared_workload: { ...sharedFamily, source: String((tiny as any).throughput_source || '').trim() || null },
    rich_status_path_readiness: readinessFamily,
  };
  if (
    sanitizedReport.infring_measured &&
    typeof sanitizedReport.infring_measured === 'object'
  ) {
    assignKernelSharedThroughputMetric(sanitizedReport.infring_measured);
    (sanitizedReport.infring_measured as any).rich_end_to_end_command_path_ops_per_sec =
      (rich as any).rich_end_to_end_command_path_ops_per_sec ?? null;
    (sanitizedReport.infring_measured as any).rich_end_to_end_command_path_source =
      (rich as any).rich_end_to_end_command_path_source ?? null;
    (sanitizedReport.infring_measured as any).benchmark_metric_families =
      (rich as any).benchmark_metric_families ?? null;
  }

  writeJson(options.reportPath, sanitizedReport);
  if (options.mirrorLegacy) {
    ensureParent(LEGACY_REPORT_PATH);
    copyFileSync(resolve(ROOT, options.reportPath), resolve(ROOT, LEGACY_REPORT_PATH));
    ensureParent(LEGACY_FULL_INSTALL_REPORT_PATH);
    copyFileSync(resolve(ROOT, options.reportPath), resolve(ROOT, LEGACY_FULL_INSTALL_REPORT_PATH));
  }

  const readmePath = 'README.md';
  const readmeBefore = readText(readmePath);
  const readmeAfter = upsertBenchmarkSnapshotBlock(
    readmeBefore,
    renderBenchmarkSnapshotBlock(sanitizedReport)
  );
  if (readmeAfter !== readmeBefore) {
    writeText(readmePath, readmeAfter);
  }

  console.log(
    JSON.stringify(
      {
        ok: true,
        type: 'benchmark_matrix_refresh',
        report_path: options.reportPath,
        mirrored_legacy_path: options.mirrorLegacy ? LEGACY_REPORT_PATH : null,
        mirrored_legacy_full_install_path: options.mirrorLegacy ? LEGACY_FULL_INSTALL_REPORT_PATH : null,
        build_profile: profileDir,
        canonical_throughput_metric: CANONICAL_THROUGHPUT_METRIC,
        preflight_attempts: attempts.length,
        preflight_history: attempts.map((row) => ({
          attempt: row.attempt,
          ok: row.ok,
          blockers: row.blockers
        })),
        rich: rich ? true : false,
        rich_engine_start_ms: richEngineStartMs,
        rich_gateway_supervisor_orchestration_ms: richGatewaySupervisorOrchestrationMs,
        rich_kernel_ready_ms: richKernelReadyMs,
        rich_gateway_ready_ms: richGatewayReadyMs,
        rich_dashboard_interactive_ms: richDashboardInteractiveMs,
        rich_end_to_end_command_path_ops_per_sec:
          (rich as any).rich_end_to_end_command_path_ops_per_sec ?? null,
        rich_end_to_end_command_path_source:
          (rich as any).rich_end_to_end_command_path_source ?? null,
        pure: pure ? true : false,
        tiny_max: tiny ? true : false
      },
      null,
      2
    )
  );
}

main();
