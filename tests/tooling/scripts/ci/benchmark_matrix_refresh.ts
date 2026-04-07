#!/usr/bin/env node
/* eslint-disable no-console */
import { copyFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const ROOT = resolve(__dirname, '..', '..', '..', '..');
const DEFAULT_REPORT_PATH = 'docs/client/reports/benchmark_matrix_run_latest.json';
const LEGACY_REPORT_PATH = 'docs/client/reports/benchmark_matrix_run_2026-03-06.json';
const LEGACY_FULL_INSTALL_REPORT_PATH = 'docs/client/reports/benchmark_matrix_run_2026-03-06_full_install.json';

type Options = {
  retries: number;
  retryDelayMs: number;
  reportPath: string;
  mirrorLegacy: boolean;
  release: boolean;
  throughputUncached: boolean;
  refreshRuntime: boolean;
  preflightMaxLoadPerCore: number;
  preflightMaxNoiseCvPct: number;
  preflightNoiseSampleMs: number;
  preflightNoiseRounds: number;
};

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
    release: true,
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
    if (key === 'release') opts.release = parseBool(value, opts.release);
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
    env: { ...process.env, PROTHEUS_ROOT: ROOT }
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

function buildArgs(manifestPath: string, bin: string, release: boolean): string[] {
  const args = ['build', '--quiet', '--manifest-path', manifestPath, '--bin', bin];
  if (release) args.splice(2, 0, '--release');
  return args;
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
  const profileDir = options.release ? 'release' : 'debug';
  const protheusOpsBin = `target/${profileDir}/protheus-ops`;

  assertRunOk(
    options.release ? 'build_protheus_ops_release' : 'build_protheus_ops_debug',
    'cargo',
    buildArgs('core/layer0/ops/Cargo.toml', 'protheus-ops', options.release)
  );
  assertRunOk(
    options.release ? 'build_protheusd_release' : 'build_protheusd_debug',
    'cargo',
    buildArgs('core/layer0/ops/Cargo.toml', 'protheusd', options.release)
  );
  assertRunOk(
    options.release ? 'build_pure_workspace_release' : 'build_pure_workspace_debug',
    'cargo',
    buildArgs('client/pure-workspace/Cargo.toml', 'protheus-pure-workspace', options.release)
  );

  const attempts: Array<{ attempt: number; ok: boolean; blockers: string[] }> = [];
  let preflightOk = false;
  for (let attempt = 1; attempt <= options.retries; attempt += 1) {
    const proc = run(protheusOpsBin, benchmarkArgs('status', options));
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

  const runProc = run(protheusOpsBin, benchmarkArgs('run', options));
  if ((runProc.status ?? 1) !== 0) {
    throw new Error(
      `benchmark_matrix_run_failed:status=${runProc.status}\nstdout=${String(runProc.stdout || '').trim()}\nstderr=${String(runProc.stderr || '').trim()}`
    );
  }
  const report = parseJsonStdout(runProc.stdout);
  if (!report || report.ok !== true) {
    throw new Error('benchmark_matrix_run_invalid_json');
  }
  report.benchmark_refresh_context = {
    build_profile: profileDir,
    refresh_script: 'tests/tooling/scripts/ci/benchmark_matrix_refresh.ts',
    throughput_uncached: options.throughputUncached,
    refresh_runtime: options.refreshRuntime
  };

  const projects = report.projects && typeof report.projects === 'object' ? report.projects : {};
  const rich = projects['InfRing (rich)'] || projects.Infring || report.infring_measured;
  const pure = projects['InfRing (pure)'] || report.pure_workspace_measured;
  const tiny = projects['InfRing (tiny-max)'] || report.pure_workspace_tiny_max_measured;
  if (!rich || !pure || !tiny) {
    throw new Error(
      `benchmark_matrix_modes_missing:rich=${!!rich},pure=${!!pure},tiny_max=${!!tiny}`
    );
  }
  const richEngineStartMs = Number((rich as any).engine_start_ms);
  const richGatewaySupervisorOrchestrationMs = Number(
    (rich as any).gateway_supervisor_orchestration_ms
  );
  if (!Number.isFinite(richEngineStartMs) || !Number.isFinite(richGatewaySupervisorOrchestrationMs)) {
    throw new Error(
      `benchmark_matrix_rich_startup_breakdown_missing:engine_start_ms=${String(
        (rich as any).engine_start_ms
      )},gateway_supervisor_orchestration_ms=${String(
        (rich as any).gateway_supervisor_orchestration_ms
      )}`
    );
  }

  writeJson(options.reportPath, report);
  if (options.mirrorLegacy) {
    ensureParent(LEGACY_REPORT_PATH);
    copyFileSync(resolve(ROOT, options.reportPath), resolve(ROOT, LEGACY_REPORT_PATH));
    ensureParent(LEGACY_FULL_INSTALL_REPORT_PATH);
    copyFileSync(resolve(ROOT, options.reportPath), resolve(ROOT, LEGACY_FULL_INSTALL_REPORT_PATH));
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
        preflight_attempts: attempts.length,
        preflight_history: attempts.map((row) => ({
          attempt: row.attempt,
          ok: row.ok,
          blockers: row.blockers
        })),
        rich: rich ? true : false,
        rich_engine_start_ms: richEngineStartMs,
        rich_gateway_supervisor_orchestration_ms: richGatewaySupervisorOrchestrationMs,
        pure: pure ? true : false,
        tiny_max: tiny ? true : false
      },
      null,
      2
    )
  );
}

main();
