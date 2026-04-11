#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-169
 * Public modular API for live core kernel surfaces.
 */

const fs = require('node:fs');
const path = require('node:path');
const { invokeTsModuleSync } = require('../../client/runtime/lib/in_process_ts_delegate.ts');

const ROOT = path.join(__dirname, '..', '..');
const CORE_PACKAGE_DIR = __dirname;
const OPS_BRIDGE = path.join(ROOT, 'adapters', 'runtime', 'run_protheus_ops.ts');
const REFLEX_BRIDGE = path.join(ROOT, 'client', 'cognition', 'habits', 'scripts', 'reflex_habit_bridge.ts');

function parseJsonPayload(stdout: string) {
  const text = String(stdout || '').trim();
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {}
  const lines = text.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {}
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
    : parseJsonPayload(stdout);
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

function runReflex(args: string[] = []) {
  return runTsExport(REFLEX_BRIDGE, 'run', args);
}

function spineStatus(extraArgs: string[] = []) {
  return runOps(['spine', 'status'].concat(Array.isArray(extraArgs) ? extraArgs : []));
}

function reflexStatus(extraArgs: string[] = []) {
  return runReflex(['status'].concat(Array.isArray(extraArgs) ? extraArgs : []));
}

function gateStatus(extraArgs: string[] = []) {
  return runOps(['security-plane', 'status'].concat(Array.isArray(extraArgs) ? extraArgs : []));
}

function toBoolOption(v: unknown, fallback = true) {
  if (v == null) return fallback;
  const raw = String(v).trim().toLowerCase();
  if (['1', 'true', 'yes', 'on'].includes(raw)) return true;
  if (['0', 'false', 'no', 'off'].includes(raw)) return false;
  return fallback;
}

function coreStatus(options: Record<string, any> = {}) {
  const includeSpine = toBoolOption(options.spine, true);
  const includeReflex = toBoolOption(options.reflex, true);
  const includeGates = toBoolOption(options.gates, true);
  const out: Record<string, any> = {
    ok: true,
    starter: 'protheus-core-live',
    runtime_contract: {
      spine: 'infring-ops spine status',
      reflex: 'client/cognition/habits/scripts/reflex_habit_bridge.ts status',
      gates: 'infring-ops security-plane status',
    },
    flags: {
      spine: includeSpine,
      reflex: includeReflex,
      gates: includeGates,
    },
  };
  if (includeSpine) out.spine = spineStatus();
  if (includeReflex) out.reflex = reflexStatus();
  if (includeGates) out.gates = gateStatus();
  out.ok = ['spine', 'reflex', 'gates']
    .filter((key) => Object.prototype.hasOwnProperty.call(out, key))
    .every((key) => out[key] && out[key].ok === true);
  return out;
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
    for (const name of names) {
      stack.push(path.join(current, name));
    }
  }
  return total;
}

function coldStartContract(options: Record<string, any> = {}) {
  const packageBytes = folderSizeBytes(CORE_PACKAGE_DIR);
  const budgetMb = Number(options.max_mb || options.maxMb || 5);
  const budgetMs = Number(options.max_ms || options.maxMs || 200);
  const started = process.hrtime.bigint();
  const boot = coreStatus(options);
  const elapsedMs = Number(process.hrtime.bigint() - started) / 1_000_000;
  return {
    ok: boot.ok === true && (packageBytes / (1024 * 1024)) <= budgetMb && elapsedMs <= budgetMs,
    package_size_bytes: packageBytes,
    package_size_mb: Number((packageBytes / (1024 * 1024)).toFixed(6)),
    cold_start_ms: Number(elapsedMs.toFixed(3)),
    budgets: {
      max_mb: budgetMb,
      max_ms: budgetMs,
    },
    boot,
  };
}

module.exports = {
  spineStatus,
  reflexStatus,
  gateStatus,
  coreStatus,
  coldStartContract,
};
