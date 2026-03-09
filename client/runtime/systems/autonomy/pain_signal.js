#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/autonomy + core/layer0/ops::autonomy-controller (authoritative)
// Core-first pain-signal command path with TS fallback.
const path = require('path');
const { spawnSync } = require('child_process');
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge');
const tsBootstrap = require('../../lib/ts_bootstrap');

const bridge = createOpsLaneBridge(__dirname, 'autonomy_controller', 'autonomy-controller');
const ROOT = path.resolve(__dirname, '..', '..');
const TS_ENTRYPOINT = path.join(ROOT, 'lib', 'ts_entrypoint.js');
const TS_TARGET = path.join(__dirname, 'pain_signal.ts');

process.env.PROTHEUS_CONDUIT_STARTUP_PROBE = '0';
process.env.PROTHEUS_CONDUIT_COMPAT_FALLBACK = '0';
process.env.PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS || '15000';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '20000';

function toCoreArgs(argv = []) {
  const args = Array.isArray(argv) ? argv.slice() : [];
  const cmd = String(args[0] || 'status').trim().toLowerCase();
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') {
    return { useCore: true, coreArgs: ['pain-signal', '--action=help'], legacyArgs: ['help'] };
  }
  if (cmd === 'status' || cmd === 'emit' || cmd === 'focus-start' || cmd === 'focus-stop' || cmd === 'focus-status') {
    return {
      useCore: true,
      coreArgs: ['pain-signal', `--action=${cmd}`, ...args.slice(1)],
      legacyArgs: args
    };
  }
  return { useCore: false, coreArgs: [], legacyArgs: args };
}

function runLegacy(args = []) {
  const run = spawnSync(process.execPath, [TS_ENTRYPOINT, TS_TARGET, ...(Array.isArray(args) ? args : [])], {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: Number(process.env.PROTHEUS_PAIN_SIGNAL_TS_TIMEOUT_MS || 120000),
    env: process.env
  });
  return {
    status: Number.isFinite(run && run.status) ? Number(run.status) : 1,
    stdout: String((run && run.stdout) || ''),
    stderr: String((run && run.stderr) || ''),
    payload: null
  };
}

function runCore(coreArgs = []) {
  const out = bridge.run(Array.isArray(coreArgs) ? coreArgs : []);
  if (out && out.status === 0) {
    if (out.stdout) process.stdout.write(out.stdout);
    if (out.stderr) process.stderr.write(out.stderr);
    if (out.payload && !out.stdout) process.stdout.write(`${JSON.stringify(out.payload)}\n`);
    return out;
  }
  return null;
}

if (require.main === module) {
  const raw = process.argv.slice(2);
  const mapped = toCoreArgs(raw);
  if (mapped.useCore) {
    const out = runCore(mapped.coreArgs);
    if (out) process.exit(0);
  }
  const fallback = runLegacy(mapped.legacyArgs);
  if (fallback.stdout) process.stdout.write(fallback.stdout);
  if (fallback.stderr) process.stderr.write(fallback.stderr);
  process.exit(Number.isFinite(fallback.status) ? Number(fallback.status) : 1);
}

if (require.main !== module) {
  tsBootstrap.bootstrap(__filename, module);
}
