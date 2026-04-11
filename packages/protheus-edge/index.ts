#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-189 package contract.
 * Compatibility facade for the supported mobile/edge subset.
 */

const fs = require('node:fs');
const path = require('node:path');
const { invokeTsModuleSync } = require('../../client/runtime/lib/in_process_ts_delegate.ts');

const ROOT = path.join(__dirname, '..', '..');
const EDGE_PACKAGE_DIR = __dirname;
const OPS_BRIDGE = path.join(ROOT, 'adapters', 'runtime', 'run_protheus_ops.ts');
const MOBILE_ADAPTER = path.join(ROOT, 'client', 'runtime', 'systems', 'hybrid', 'mobile', 'protheus_mobile_adapter.ts');
const WRAPPER_POLICY = path.join(ROOT, 'client', 'runtime', 'config', 'mobile_wrapper_distribution_pack_policy.json');
const WRAPPER_TARGETS = [
  {
    id: 'android_termux',
    dir: path.join(EDGE_PACKAGE_DIR, 'wrappers', 'android_termux'),
  },
  {
    id: 'ios_tauri',
    dir: path.join(EDGE_PACKAGE_DIR, 'wrappers', 'ios_tauri'),
  },
];

function parseJson(stdout: string) {
  const text = String(stdout || '').trim();
  if (!text) return null;
  try { return JSON.parse(text); } catch {}
  const lines = text.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try { return JSON.parse(lines[i]); } catch {}
  }
  return null;
}

function normalizeDelegateResult(out: any) {
  const value = out && typeof out.value === 'object' ? out.value : null;
  const status = Number.isFinite(Number(value && value.status))
    ? Number(value.status)
    : Number.isFinite(Number(out && out.status))
      ? Number(out.status)
      : 1;
  const stdout = value && typeof value.stdout === 'string'
    ? value.stdout
    : String((out && out.stdout) || '');
  const stderr = value && typeof value.stderr === 'string'
    ? value.stderr
    : String((out && out.stderr) || '');
  const payload = value && value.payload && typeof value.payload === 'object'
    ? value.payload
    : parseJson(stdout);
  return {
    ok: status === 0,
    status,
    stdout,
    stderr,
    payload,
  };
}

function runTsExport(scriptAbs: string, exportName: string, args: string[] = []) {
  const out = invokeTsModuleSync(scriptAbs, {
    argv: Array.isArray(args) ? args.map((value) => String(value)) : [],
    cwd: ROOT,
    exportName,
    teeStdout: false,
    teeStderr: false,
  });
  return normalizeDelegateResult(out);
}

function runOps(args: string[] = []) {
  return runTsExport(OPS_BRIDGE, 'invokeProtheusOpsViaBridge', args);
}

function runMobileAdapter(args: string[] = []) {
  return runTsExport(MOBILE_ADAPTER, 'run', args);
}

function toFlags(options: Record<string, any> = {}) {
  const out: string[] = [];
  for (const [k, v] of Object.entries(options || {})) {
    if (v == null) continue;
    const key = String(k).replace(/[A-Z]/g, (m) => `-${m.toLowerCase()}`);
    out.push(`--${key}=${String(v)}`);
  }
  return out;
}

function toBoolOption(v: unknown, fallback = true) {
  if (v == null) return fallback;
  const raw = String(v).trim().toLowerCase();
  if (['1', 'true', 'yes', 'on'].includes(raw)) return true;
  if (['0', 'false', 'no', 'off'].includes(raw)) return false;
  return fallback;
}

function compatibilityStub(surface: string, command: string, reasonCode: string, recommendedSurface: string) {
  const payload = {
    ok: false,
    type: 'protheus_edge_compat_notice',
    surface,
    command,
    deprecated: true,
    supported: false,
    reason_code: reasonCode,
    recommended_surface: recommendedSurface,
  };
  return {
    ok: false,
    status: 64,
    stdout: `${JSON.stringify(payload)}\n`,
    stderr: `${reasonCode}\n`,
    payload,
  };
}

function inspectWrapperTarget(target: { id: string; dir: string }) {
  const installPath = path.join(target.dir, 'install.sh');
  const runPath = path.join(target.dir, 'run.sh');
  const verifyPath = path.join(target.dir, 'verify.sh');
  return {
    target: target.id,
    dir: path.relative(ROOT, target.dir).replace(/\\/g, '/'),
    exists: fs.existsSync(target.dir),
    install_script: fs.existsSync(installPath),
    run_script: fs.existsSync(runPath),
    verify_script: fs.existsSync(verifyPath),
  };
}

function edgeRuntime(command: string, options: Record<string, any> = {}) {
  const normalized = String(command || 'status').trim().toLowerCase() || 'status';
  if (normalized !== 'status') {
    return compatibilityStub(
      'edgeRuntime',
      normalized,
      'edge_runtime_start_removed',
      'client/runtime/systems/hybrid/mobile/protheus_mobile_adapter.ts status --json',
    );
  }
  return runMobileAdapter(['status', '--json'].concat(toFlags(options)));
}

function edgeLifecycle(command: string, options: Record<string, any> = {}) {
  const op = String(command || 'status').trim().toLowerCase() || 'status';
  return runOps(['persist-plane', 'mobile-daemon', `--op=${op}`].concat(toFlags(options)));
}

function edgeSwarm(command: string, options: Record<string, any> = {}) {
  void options;
  return compatibilityStub(
    'edgeSwarm',
    String(command || 'status').trim().toLowerCase() || 'status',
    'edge_swarm_bridge_removed',
    'client/runtime/systems/autonomy/swarm_sessions_bridge.ts',
  );
}

function edgeWrapper(command: string, options: Record<string, any> = {}) {
  const normalized = String(command || 'status').trim().toLowerCase() || 'status';
  if (normalized !== 'status') {
    return compatibilityStub(
      'edgeWrapper',
      normalized,
      'edge_wrapper_distribution_runtime_removed',
      'packages/protheus-edge/wrappers/* + packages/protheus-edge/starter.ts --mode=status',
    );
  }
  const requestedTarget = String(options.target || '').trim().toLowerCase();
  const targets = WRAPPER_TARGETS
    .filter((target) => !requestedTarget || target.id === requestedTarget)
    .map(inspectWrapperTarget);
  const payload = {
    ok: targets.length > 0 && targets.every((target) => target.exists && target.install_script && target.run_script && target.verify_script),
    type: 'protheus_edge_wrapper_status',
    supported: true,
    verification_mode: 'static_wrapper_contract',
    policy_path: path.relative(ROOT, WRAPPER_POLICY).replace(/\\/g, '/'),
    policy_present: fs.existsSync(WRAPPER_POLICY),
    targets,
  };
  return {
    ok: payload.ok,
    status: payload.ok ? 0 : 1,
    stdout: `${JSON.stringify(payload)}\n`,
    stderr: payload.ok ? '' : 'protheus_edge_wrapper_status_failed\n',
    payload,
  };
}

function edgeBenchmark(command: string, options: Record<string, any> = {}) {
  const normalized = String(command || 'status').trim().toLowerCase() || 'status';
  return runOps(['benchmark-matrix', normalized].concat(toFlags(options)));
}

function mobileCockpitStatus(options: Record<string, any> = {}) {
  return runOps(['persist-plane', 'mobile-cockpit', '--op=status'].concat(toFlags(options)));
}

function mobileTop(options: Record<string, any> = {}) {
  return edgeRuntime('status', options);
}

function folderSizeBytes(dirPath: string) {
  if (!fs.existsSync(dirPath)) return 0;
  const stack = [dirPath];
  let total = 0;
  while (stack.length > 0) {
    const current = stack.pop();
    if (!current) continue;
    const stat = fs.statSync(current);
    if (stat.isFile()) {
      total += Number(stat.size || 0);
      continue;
    }
    const names = fs.readdirSync(current);
    for (const name of names) stack.push(path.join(current, name));
  }
  return total;
}

function edgeStatusBundle(options: Record<string, any> = {}) {
  const includeEdge = toBoolOption(options.edge, true);
  const includeLifecycle = toBoolOption(options.lifecycle, true);
  const includeCockpit = toBoolOption(options.cockpit, true);
  const includeWrappers = toBoolOption(options.wrappers, true);
  const includeBenchmark = toBoolOption(options.benchmark, true);
  const includeTop = toBoolOption(options.top, true);
  const bundle = {
    ...(includeEdge ? { edge: edgeRuntime('status', options) } : {}),
    ...(includeLifecycle ? { lifecycle: edgeLifecycle('status', options) } : {}),
    ...(includeCockpit ? { cockpit: mobileCockpitStatus(options) } : {}),
    ...(includeWrappers ? { wrappers: edgeWrapper('status', options) } : {}),
    ...(includeBenchmark ? { benchmark: edgeBenchmark('status', options) } : {}),
    ...(includeTop ? { top: mobileTop(options) } : {}),
    compatibility: {
      swarm: edgeSwarm('status', options).payload,
      note: 'Removed edge/swarm wrapper-era paths are retained only as explicit compatibility notices.',
    },
  };
  return {
    ok: ['edge', 'lifecycle', 'cockpit', 'wrappers', 'benchmark', 'top']
      .filter((key) => Object.prototype.hasOwnProperty.call(bundle, key))
      .every((key) => bundle[key] && bundle[key].ok === true),
    supported_surface: [
      'protheus_mobile_adapter',
      'persist-plane mobile-cockpit',
      'persist-plane mobile-daemon',
      'benchmark-matrix',
      'wrapper static contract',
    ],
    ...bundle,
  };
}

function edgeContract(options: Record<string, any> = {}) {
  const packageBytes = folderSizeBytes(EDGE_PACKAGE_DIR);
  const budgetMb = Number(options.max_mb || options.maxMb || 5);
  const budgetMs = Number(options.max_ms || options.maxMs || 200);
  const started = process.hrtime.bigint();
  const run = edgeRuntime('status', options);
  const elapsedMs = Number(process.hrtime.bigint() - started) / 1_000_000;
  return {
    ok: run.ok === true && (packageBytes / (1024 * 1024)) <= budgetMb && elapsedMs <= budgetMs,
    package_size_bytes: packageBytes,
    package_size_mb: Number((packageBytes / (1024 * 1024)).toFixed(6)),
    cold_start_ms: Number(elapsedMs.toFixed(3)),
    budgets: {
      max_mb: budgetMb,
      max_ms: budgetMs,
    },
    run,
  };
}

module.exports = {
  edgeRuntime,
  edgeLifecycle,
  edgeSwarm,
  edgeWrapper,
  edgeBenchmark,
  mobileCockpitStatus,
  mobileTop,
  edgeStatusBundle,
  edgeContract,
};
