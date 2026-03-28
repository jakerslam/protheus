#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
const bridge = createOpsLaneBridge(
  __dirname,
  'security_layer_inventory_gate',
  'security-layer-inventory-gate-kernel'
);

function normalizePayload(out) {
  const payload = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  if (payload && typeof payload === 'object') return payload;
  const stderr = out && typeof out.stderr === 'string' ? out.stderr.trim() : '';
  return {
    ok: false,
    type: 'security_layer_inventory_gate',
    error: stderr || 'security_layer_inventory_gate_kernel_bridge_failed'
  };
}

function run(argv = []) {
  const args = Array.isArray(argv) ? argv.map((token) => String(token || '')) : [];
  const out = bridge.run(args);
  return normalizePayload(out);
}

module.exports = { run };

if (require.main === module) {
  bridge.runCli(process.argv.slice(2));
}
