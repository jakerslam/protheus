#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/ops (authoritative), with legacy TS fallback.
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge');
process.env.PROTHEUS_CONDUIT_STARTUP_PROBE = '0';
process.env.PROTHEUS_CONDUIT_COMPAT_FALLBACK = '0';

const healthBridge = createOpsLaneBridge(__dirname, 'health_status', 'health-status');
const adaptiveBridge = createOpsLaneBridge(__dirname, 'adaptive_runtime', 'adaptive-runtime');

function cmdToken(argv) {
  return String((argv && argv[0]) || 'status').trim().toLowerCase();
}

function shouldFallback(out) {
  const reason = String((out && out.payload && out.payload.reason) || out && out.stderr || '');
  return /conduit_/i.test(reason) || /timeout/i.test(reason);
}

function emitAndExit(out) {
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  process.exit(Number.isFinite(out && out.status) ? Number(out.status) : 1);
}

function runCoreOrFallback(argv) {
  const cmd = cmdToken(argv);
  if (cmd === 'status' || cmd === 'diagnostics') {
    const out = healthBridge.run(['status']);
    if ((Number(out && out.status) || 1) !== 0 && shouldFallback(out)) {
      return require('../../lib/ts_bootstrap').bootstrap(__filename, module);
    }
    return emitAndExit(out);
  }
  if (cmd === 'tick') {
    const out = adaptiveBridge.run(['tick']);
    if ((Number(out && out.status) || 1) !== 0 && shouldFallback(out)) {
      return require('../../lib/ts_bootstrap').bootstrap(__filename, module);
    }
    return emitAndExit(out);
  }
  // Keep full legacy daemon lifecycle commands (start/stop/restart/attach/subscribe)
  // on the existing TS runtime until full Rust parity is complete.
  return require('../../lib/ts_bootstrap').bootstrap(__filename, module);
}

if (require.main === module) {
  runCoreOrFallback(process.argv.slice(2));
}

module.exports = {
  run: (args = []) => {
    const cmd = cmdToken(args);
    if (cmd === 'status' || cmd === 'diagnostics') return healthBridge.run(['status']);
    if (cmd === 'tick') return adaptiveBridge.run(['tick']);
    return null;
  }
};
