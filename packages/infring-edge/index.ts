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
const { sanitizeBridgeArg } = require('../../client/runtime/lib/runtime_system_entrypoint.ts');

const ROOT = path.join(__dirname, '..', '..');
const EDGE_PACKAGE_DIR = __dirname;
const OPS_BRIDGE = path.join(ROOT, 'adapters', 'runtime', 'run_infring_ops.ts');
const MOBILE_ADAPTER = path.join(ROOT, 'client', 'runtime', 'systems', 'hybrid', 'mobile', 'infring_mobile_adapter.ts');
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
const MAX_ARG_COUNT = 64;
const MAX_ARG_LENGTH = 512;
const MAX_FLAG_COUNT = 48;

function sanitizeCliToken(value: unknown, fallback = '') {
  const normalized = sanitizeBridgeArg(value == null ? fallback : value, MAX_ARG_LENGTH)
    .replace(/[\u0000-\u001f\u007f]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
  if (!normalized) return String(fallback || '');
  return normalized;
}

function sanitizeFlagKey(value: unknown) {
  const normalized = String(value == null ? '' : value)
    .replace(/[\u0000-\u001f\u007f]/g, '')
    .replace(/[A-Z]/g, (m) => `-${m.toLowerCase()}`)
    .replace(/[^a-z0-9_-]/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '')
    .trim();
  return normalized;
}

function sanitizeArgv(args: string[] = []) {
  if (!Array.isArray(args)) return [];
  const out: string[] = [];
  for (const item of args) {
    if (out.length >= MAX_ARG_COUNT) break;
    const token = sanitizeCliToken(item, '');
    if (!token) continue;
    out.push(token);
  }
  return out;
}

function isPathInsideRoot(absPath: string) {
  const rel = path.relative(ROOT, absPath);
  return rel === '' || (!rel.startsWith('..') && !path.isAbsolute(rel));
}

function bridgeFailure(scriptAbs: string, exportName: string, code: string, detail = '') {
  const payload = {
    ok: false,
    type: 'infring_edge_bridge_error',
    error: code,
    script: path.relative(ROOT, scriptAbs).replace(/\\/g, '/'),
    export_name: exportName,
    detail,
  };
  return {
    ok: false,
    status: 1,
    stdout: '',
    stderr: `${code}\n`,
    payload,
  };
}

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

function parseFiniteBoundedNumber(value: unknown, fallback: number, min: number, max: number) {
  const n = Number(value);
  if (!Number.isFinite(n)) return fallback;
  return Math.min(max, Math.max(min, n));
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
  if (!isPathInsideRoot(scriptAbs)) {
    return bridgeFailure(scriptAbs, exportName, 'bridge_script_outside_root');
  }
  const safeExportName = sanitizeCliToken(exportName, '').replace(/[^A-Za-z0-9_]/g, '');
  if (!safeExportName) {
    return bridgeFailure(scriptAbs, exportName, 'bridge_export_invalid');
  }
  try {
    const out = invokeTsModuleSync(scriptAbs, {
      argv: sanitizeArgv(args),
      cwd: ROOT,
      exportName: safeExportName,
      teeStdout: false,
      teeStderr: false,
    });
    return normalizeDelegateResult(out);
  } catch (error: any) {
    return bridgeFailure(
      scriptAbs,
      safeExportName,
      'bridge_invoke_failed',
      sanitizeCliToken(error && error.message ? error.message : 'unknown_bridge_error', ''),
    );
  }
}

function runOps(args: string[] = []) {
  return runTsExport(OPS_BRIDGE, 'invokeInfringOpsViaBridge', args);
}

function runMobileAdapter(args: string[] = []) {
  return runTsExport(MOBILE_ADAPTER, 'run', args);
}

function toFlags(options: Record<string, any> = {}) {
  const out: string[] = [];
  for (const [k, v] of Object.entries(options || {})) {
    if (out.length >= MAX_FLAG_COUNT) break;
    if (v == null) continue;
    const key = sanitizeFlagKey(k);
    if (!key) continue;
    const value = sanitizeCliToken(v, '');
    if (!value) continue;
    out.push(`--${key}=${value}`);
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
    type: 'infring_edge_compat_notice',
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
      'client/runtime/systems/hybrid/mobile/infring_mobile_adapter.ts status --json',
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
      'packages/infring-edge/wrappers/* + packages/infring-edge/starter.ts --mode=status',
    );
  }
  const requestedTarget = sanitizeCliToken(options.target || '', '').toLowerCase();
  if (options.target != null && !requestedTarget) {
    const payload = {
      ok: false,
      type: 'infring_edge_wrapper_status',
      supported: true,
      error: 'edge_wrapper_target_invalid',
      targets: [],
    };
    return {
      ok: false,
      status: 1,
      stdout: `${JSON.stringify(payload)}\n`,
      stderr: 'edge_wrapper_target_invalid\n',
      payload,
    };
  }
  const targets = WRAPPER_TARGETS
    .filter((target) => !requestedTarget || target.id === requestedTarget)
    .map(inspectWrapperTarget);
  const payload = {
    ok: targets.length > 0 && targets.every((target) => target.exists && target.install_script && target.run_script && target.verify_script),
    type: 'infring_edge_wrapper_status',
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
    stderr: payload.ok ? '' : 'infring_edge_wrapper_status_failed\n',
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
  const evaluatedSurfaces = ['edge', 'lifecycle', 'cockpit', 'wrappers', 'benchmark', 'top']
    .filter((key) => Object.prototype.hasOwnProperty.call(bundle, key));
  const failingSurfaces = evaluatedSurfaces
    .filter((key) => !(bundle[key] && bundle[key].ok === true));
  return {
    ok: failingSurfaces.length === 0,
    surface_count: evaluatedSurfaces.length,
    failing_surfaces: failingSurfaces,
    generated_at: new Date().toISOString(),
    contract_version: '2026-04-20',
    supported_surface: [
      'infring_mobile_adapter',
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
  const budgetMb = parseFiniteBoundedNumber(options.max_mb || options.maxMb, 5, 0.25, 256);
  const budgetMs = parseFiniteBoundedNumber(options.max_ms || options.maxMs, 200, 20, 60_000);
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
