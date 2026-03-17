#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
const bridge = createOpsLaneBridge(__dirname, 'benchmark_autonomy_gate', 'benchmark-autonomy-gate', {
  inheritStdio: false
});

function normalizePayload(out, command) {
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  if (receipt && receipt.payload && typeof receipt.payload === 'object') {
    return receipt.payload;
  }
  return {
    ok: false,
    type: 'benchmark_autonomy_gate',
    command,
    error: out && out.stderr ? String(out.stderr).trim() || 'benchmark_autonomy_gate_bridge_failed' : 'benchmark_autonomy_gate_bridge_failed',
    fail_closed: true
  };
}

function run(args = []) {
  const argv = Array.isArray(args) ? args.map((arg) => String(arg)) : [];
  const command = String(argv[0] || 'run').trim().toLowerCase() || 'run';
  const out = bridge.run(argv.length ? argv : ['run']);
  return normalizePayload(out, command);
}

module.exports = { run };

if (require.main === module) {
  const result = run(process.argv.slice(2));
  process.stdout.write(`${JSON.stringify(result)}\n`);
  if (!result.ok) process.exit(2);
}
