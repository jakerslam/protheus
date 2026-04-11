#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const ROOT = process.cwd();
const ENTRYPOINT = path.join(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const CLOSURE_POLICY_PATH = path.join(ROOT, 'client/runtime/config/production_readiness_closure_policy.json');
const RUNNER_PATH = path.join(ROOT, 'adapters/runtime/run_protheus_ops.ts');
const BRIDGE_PATH = path.join(ROOT, 'adapters/runtime/ops_lane_bridge.ts');
const DEFAULT_OUT = path.join(ROOT, 'core/local/artifacts/production_topology_diagnostic_current.json');

function clean(value: unknown, max = 500): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function parseBool(raw: string | undefined, fallback = false): boolean {
  const value = clean(raw, 24).toLowerCase();
  if (!value) return fallback;
  return value === '1' || value === 'true' || value === 'yes' || value === 'on';
}

function parseArgs(argv: string[]) {
  const parsed = {
    strict: false,
    out: DEFAULT_OUT,
  };
  for (const tokenRaw of argv) {
    const token = clean(tokenRaw, 400);
    if (!token) continue;
    if (token.startsWith('--strict=')) parsed.strict = parseBool(token.slice(9), false);
    else if (token.startsWith('--out=')) parsed.out = path.resolve(ROOT, clean(token.slice(6), 400));
  }
  return parsed;
}

function readJson<T>(filePath: string, fallback: T): T {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8')) as T;
  } catch {
    return fallback;
  }
}

function parseJsonLine(stdout: string): any {
  const whole = String(stdout || '').trim();
  if (whole) {
    try {
      return JSON.parse(whole);
    } catch {}
  }
  const lines = String(stdout || '')
    .split('\n')
    .map((row) => row.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {}
  }
  return null;
}

function runTs(scriptRelPath: string, args: string[]) {
  const out = spawnSync(process.execPath, [ENTRYPOINT, path.join(ROOT, scriptRelPath), ...args], {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env },
    maxBuffer: 32 * 1024 * 1024,
  });
  return {
    status: Number.isFinite(out.status) ? Number(out.status) : 1,
    payload: parseJsonLine(String(out.stdout || '')),
    stderr: clean(out.stderr, 800),
  };
}

function buildReport() {
  const closurePolicy = readJson<any>(CLOSURE_POLICY_PATH, {});
  const topology = runTs('client/runtime/systems/ops/transport_topology_status.ts', ['--json=1']);
  const closure = runTs('tests/tooling/scripts/ci/production_readiness_closure_gate.ts', [
    '--strict=0',
    '--run-smoke=0',
    '--out=core/local/artifacts/production_readiness_closure_gate_current.json',
  ]);
  const dr = runTs('tests/tooling/scripts/ops/dr_gameday.ts', ['gate']);
  const runnerSource = fs.existsSync(RUNNER_PATH) ? fs.readFileSync(RUNNER_PATH, 'utf8') : '';
  const bridgeSource = fs.existsSync(BRIDGE_PATH) ? fs.readFileSync(BRIDGE_PATH, 'utf8') : '';
  const releaseChannel = clean(
    process.env.INFRING_RELEASE_CHANNEL || process.env.PROTHEUS_RELEASE_CHANNEL || 'stable',
    64,
  ).toLowerCase() || 'stable';
  const supportSurface = closurePolicy?.production_surface_contract || {};
  const legacyRunnerCodePresent =
    runnerSource.includes('spawnSync') || runnerSource.includes('legacyProcessRunnerForced');
  const legacyRunnerDevOnly = runnerSource.includes('legacy_process_runner_dev_only');
  const releaseEntrypointsQuarantined =
    !runnerSource.includes('spawnSync(') &&
    !bridgeSource.includes('spawnSync(') &&
    runnerSource.includes("./dev_only/legacy_process_runner.ts") &&
    bridgeSource.includes("./dev_only/ops_lane_process_fallback.ts");
  const closurePass = closure.payload?.summary?.pass === true || closure.payload?.ok === true;
  const topologyPass = topology.payload?.ok === true;
  const degradedFlags = []
    .concat(Array.isArray(topology.payload?.violations) ? topology.payload.violations.map((row: any) => row.id) : [])
    .concat(closurePass ? [] : ['production_closure_regressed'])
    .concat(legacyRunnerDevOnly ? [] : ['legacy_runner_not_dev_only'])
    .concat(releaseEntrypointsQuarantined ? [] : ['legacy_runner_not_quarantined_from_release_entrypoints'])
    .concat(dr.payload?.gate_state === 'fail' ? ['recovery_rehearsal_regressed'] : [])
    .concat(dr.payload?.gate_state === 'insufficient_samples' ? ['recovery_samples_insufficient'] : [])
    .filter(Boolean);
  const supportedProductionTopology =
    topologyPass && closurePass && legacyRunnerDevOnly && releaseEntrypointsQuarantined;
  const supportLevel = supportedProductionTopology
    ? 'production_supported'
    : topology.payload?.production_release === true
      ? 'degraded'
      : 'non_production_topology';

  return {
    ok: degradedFlags.length === 0 && topologyPass && closurePass && legacyRunnerDevOnly,
    type: 'production_topology_diagnostic',
    generated_at: new Date().toISOString(),
    release_channel: releaseChannel,
    topology_mode: clean(topology.payload?.topology_mode || 'resident_ipc_authoritative', 120),
    supported_production_topology: supportedProductionTopology,
    support_level: supportLevel,
    surface_contract: {
      canonical_surface: clean(supportSurface.canonical_surface || '', 80),
      constrained_profiles: Array.isArray(supportSurface.constrained_profiles)
        ? supportSurface.constrained_profiles
        : [],
      production_supported: Array.isArray(supportSurface.command_tiers?.production_supported)
        ? supportSurface.command_tiers.production_supported
        : [],
      experimental: Array.isArray(supportSurface.command_tiers?.experimental)
        ? supportSurface.command_tiers.experimental
        : [],
    },
    closure_status: {
      ok: closurePass,
      failed_ids: Array.isArray(closure.payload?.failed_ids) ? closure.payload.failed_ids : [],
    },
    transport: topology.payload?.transport || {},
    legacy_runner: {
      code_present: legacyRunnerCodePresent,
      release_allowed: false,
      dev_only_marker: legacyRunnerDevOnly,
      quarantined_from_release_entrypoints: releaseEntrypointsQuarantined,
    },
    degraded_flags: degradedFlags,
    recovery: {
      gate_state: clean(dr.payload?.gate_state || '', 80),
      sample_count: Number(dr.payload?.sample_count || 0),
      pass_rate: Number(dr.payload?.pass_rate || 0),
    },
  };
}

function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const report = buildReport();
  fs.mkdirSync(path.dirname(args.out), { recursive: true });
  fs.writeFileSync(args.out, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  process.stdout.write(`${JSON.stringify(report)}\n`);
  if (args.strict && report.ok !== true) return 1;
  return 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run, buildReport };
