#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-217
 * Windows Native Runtime Parity (Tauri + DirectML/ONNX) with deterministic fallback.
 */

const fs = require('fs');
const os = require('os');
const path = require('path');
const crypto = require('crypto');

type AnyObj = Record<string, any>;

const ROOT = process.env.WINDOWS_RUNTIME_PARITY_ROOT
  ? path.resolve(process.env.WINDOWS_RUNTIME_PARITY_ROOT)
  : path.resolve(__dirname, '..', '..');

const DEFAULT_POLICY_PATH = process.env.WINDOWS_RUNTIME_PARITY_POLICY_PATH
  ? path.resolve(process.env.WINDOWS_RUNTIME_PARITY_POLICY_PATH)
  : path.join(ROOT, 'config', 'windows_native_runtime_parity_policy.json');

function nowIso() {
  return new Date().toISOString();
}

function cleanText(v: unknown, maxLen = 260) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function parseArgs(argv: string[]) {
  const out: AnyObj = { _: [] };
  for (const tokRaw of argv) {
    const tok = String(tokRaw || '');
    if (!tok.startsWith('--')) {
      out._.push(tok);
      continue;
    }
    const idx = tok.indexOf('=');
    if (idx < 0) out[tok.slice(2)] = true;
    else out[tok.slice(2, idx)] = tok.slice(idx + 1);
  }
  return out;
}

function toBool(v: unknown, fallback = false) {
  if (v == null) return fallback;
  const raw = String(v).trim().toLowerCase();
  if (['1', 'true', 'yes', 'on'].includes(raw)) return true;
  if (['0', 'false', 'no', 'off'].includes(raw)) return false;
  return fallback;
}

function ensureDir(dirPath: string) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function readJson(filePath: string, fallback: AnyObj = {}) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    const parsed = JSON.parse(fs.readFileSync(filePath, 'utf8'));
    return parsed && typeof parsed === 'object' ? parsed : fallback;
  } catch {
    return fallback;
  }
}

function writeJsonAtomic(filePath: string, payload: AnyObj) {
  ensureDir(path.dirname(filePath));
  const tmp = `${filePath}.tmp-${Date.now()}-${process.pid}`;
  fs.writeFileSync(tmp, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
  fs.renameSync(tmp, filePath);
}

function appendJsonl(filePath: string, row: AnyObj) {
  ensureDir(path.dirname(filePath));
  fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`, 'utf8');
}

function resolvePath(raw: unknown, fallbackRel: string) {
  const txt = cleanText(raw, 520);
  if (!txt) return path.join(ROOT, fallbackRel);
  return path.isAbsolute(txt) ? txt : path.join(ROOT, txt);
}

function rel(filePath: string) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function stableHash(v: unknown, len = 16) {
  return crypto.createHash('sha256').update(String(v == null ? '' : v), 'utf8').digest('hex').slice(0, len);
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    required_capabilities: {
      tauri_shell: true,
      directml: true,
      onnx_runtime: true
    },
    fallback_runtime: 'cross_platform_runtime',
    rollback_command: 'node systems/runtime/windows_native_runtime_parity.js run --force-fallback=1',
    paths: {
      latest_path: 'state/runtime/windows_native_runtime_parity/latest.json',
      history_path: 'state/runtime/windows_native_runtime_parity/history.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const base = defaultPolicy();
  const raw = readJson(policyPath, base);
  const caps = raw.required_capabilities && typeof raw.required_capabilities === 'object' ? raw.required_capabilities : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 40) || base.version,
    enabled: raw.enabled !== false,
    required_capabilities: {
      tauri_shell: caps.tauri_shell !== false,
      directml: caps.directml !== false,
      onnx_runtime: caps.onnx_runtime !== false
    },
    fallback_runtime: cleanText(raw.fallback_runtime || base.fallback_runtime, 120) || base.fallback_runtime,
    rollback_command: cleanText(raw.rollback_command || base.rollback_command, 260) || base.rollback_command,
    paths: {
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function detectCapabilities(args: AnyObj) {
  const hostOs = cleanText(args['host-os'] || args.host_os || os.platform(), 40).toLowerCase();
  const hostArch = cleanText(args['host-arch'] || args.host_arch || os.arch(), 40).toLowerCase();

  const caps = {
    tauri_shell: toBool(args.tauri ?? process.env.TAURI_SHELL_AVAILABLE, false),
    directml: toBool(args.directml ?? process.env.DIRECTML_AVAILABLE, false),
    onnx_runtime: toBool(args.onnx ?? process.env.ONNX_RUNTIME_AVAILABLE, false)
  };

  return {
    host: {
      os: hostOs,
      arch: hostArch,
      windows_family: hostOs === 'win32' || hostOs === 'windows'
    },
    capabilities: caps
  };
}

function evaluateParity(policy: AnyObj, probe: AnyObj, forceFallback: boolean) {
  const failures: string[] = [];
  if (!probe.host.windows_family) failures.push('host_not_windows');

  const req = policy.required_capabilities || {};
  if (req.tauri_shell === true && probe.capabilities.tauri_shell !== true) failures.push('tauri_shell_missing');
  if (req.directml === true && probe.capabilities.directml !== true) failures.push('directml_missing');
  if (req.onnx_runtime === true && probe.capabilities.onnx_runtime !== true) failures.push('onnx_runtime_missing');
  if (forceFallback) failures.push('forced_fallback');

  const parityReady = failures.length === 0;
  return {
    parity_ready: parityReady,
    failures,
    selected_runtime: parityReady ? 'windows_native_runtime' : policy.fallback_runtime,
    rollback_safe_fallback: !parityReady,
    rollback_command: policy.rollback_command
  };
}

function runParity(args: AnyObj, policy: AnyObj) {
  if (policy.enabled !== true) {
    return {
      ok: true,
      type: 'windows_native_runtime_parity',
      ts: nowIso(),
      result: 'disabled_by_policy'
    };
  }

  const forceFallback = toBool(args['force-fallback'] ?? args.force_fallback, false);
  const probe = detectCapabilities(args);
  const evalOut = evaluateParity(policy, probe, forceFallback);

  return {
    ok: evalOut.parity_ready,
    ts: nowIso(),
    type: 'windows_native_runtime_parity',
    lane_id: 'V3-RACE-217',
    parity_receipt_id: `win_parity_${stableHash(JSON.stringify({ probe, evalOut }), 14)}`,
    host: probe.host,
    capabilities: probe.capabilities,
    parity_ready: evalOut.parity_ready,
    selected_runtime: evalOut.selected_runtime,
    fallback_reason_codes: evalOut.failures,
    rollback_safe_fallback: evalOut.rollback_safe_fallback,
    rollback_command: evalOut.rollback_command
  };
}

function cmdRun(args: AnyObj) {
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  const out = runParity(args, policy);
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.history_path, {
    ts: out.ts,
    type: out.type,
    ok: out.ok,
    selected_runtime: out.selected_runtime,
    fallback_reason_codes: out.fallback_reason_codes
  });
  return {
    ...out,
    policy_path: rel(policy.policy_path),
    latest_path: rel(policy.paths.latest_path)
  };
}

function cmdStatus(args: AnyObj) {
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  return {
    ok: true,
    ts: nowIso(),
    type: 'windows_native_runtime_parity_status',
    latest: readJson(policy.paths.latest_path, null),
    latest_path: rel(policy.paths.latest_path),
    policy_path: rel(policy.policy_path)
  };
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/runtime/windows_native_runtime_parity.js run [--host-os=win32 --tauri=1 --directml=1 --onnx=1] [--force-fallback=0] [--policy=<path>]');
  console.log('  node systems/runtime/windows_native_runtime_parity.js status [--policy=<path>]');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'status', 80).toLowerCase();
  if (args.help || cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    process.exit(0);
  }

  const out = cmd === 'run'
    ? cmdRun(args)
    : cmd === 'status'
      ? cmdStatus(args)
      : null;

  if (!out) {
    usage();
    process.exit(2);
  }

  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
  if (cmd === 'run' && toBool(args.strict, false) && out.ok !== true) process.exit(1);
}

if (require.main === module) {
  main();
}

module.exports = {
  loadPolicy,
  detectCapabilities,
  evaluateParity,
  runParity,
  cmdRun,
  cmdStatus
};
