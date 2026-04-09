#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'rust_hotpath_inventory', 'rust-hotpath-inventory-kernel');

function parseLastJson(stdout) {
  const lines = String(stdout || '')
    .trim()
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {}
  }
  return null;
}

function normalizePayload(out) {
  const payload = out && out.payload && typeof out.payload === 'object'
    ? out.payload
    : null;
  if (payload) return payload;
  const parsed = out && typeof out.stdout === 'string' ? parseLastJson(out.stdout) : null;
  if (parsed) return parsed;
  const stderr = out && typeof out.stderr === 'string' ? out.stderr.trim() : '';
  return {
    ok: false,
    type: 'rust_hotpath_inventory',
    error: stderr || 'rust_hotpath_inventory_kernel_bridge_failed'
  };
}

function runKernel(command, args = []) {
  const passArgs = [
    String(command || '').trim(),
    ...(Array.isArray(args) ? args : []).map((token) => String(token || '').trim()),
  ].filter(Boolean);
  return bridge.run(passArgs);
}

function buildInventory(argv = []) {
  const args = Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
  return normalizePayload(runKernel('inventory', args));
}

function run(argv = process.argv.slice(2)) {
  const args = Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
  const first = args[0] || '';
  const command = first && !first.startsWith('--') ? first.toLowerCase() : 'status';
  if (command !== 'run' && command !== 'status' && command !== 'inventory') {
    process.stderr.write('Usage: node client/runtime/systems/ops/rust_hotpath_inventory.ts <run|status|inventory> [--policy=<path>]\n');
    return 2;
  }
  const rest = first && !first.startsWith('--') ? args.slice(1) : args;

  const out = runKernel(command, rest);
  if (out && typeof out.stdout === 'string' && out.stdout.trim()) {
    process.stdout.write(out.stdout.endsWith('\n') ? out.stdout : `${out.stdout}\n`);
  } else {
    const payload = normalizePayload(out);
    process.stdout.write(`${JSON.stringify(payload)}\n`);
  }
  if (out && typeof out.stderr === 'string' && out.stderr.trim()) {
    process.stderr.write(out.stderr.endsWith('\n') ? out.stderr : `${out.stderr}\n`);
  }
  const status = out && Number.isFinite(Number(out.status)) ? Number(out.status) : 1;
  return status;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  buildInventory,
  run,
};
