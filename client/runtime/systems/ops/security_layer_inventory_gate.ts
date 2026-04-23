#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const BRIDGE_PATH = 'client/runtime/systems/ops/security_layer_inventory_gate.ts';
const KERNEL_ID = 'security-layer-inventory-gate-kernel';
const DEFAULT_COMMAND = 'run';
const bridge = createOpsLaneBridge(
  __dirname,
  'security_layer_inventory_gate',
  'security-layer-inventory-gate-kernel',
  {
    preferLocalCore: true
  }
);

function normalizeArgs(argv = []) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function extractBridgePayload(out) {
  const payload = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  if (payload && payload.payload && typeof payload.payload === 'object') return payload.payload;
  if (payload && typeof payload === 'object') return payload;
  return null;
}

function normalizePayload(out, command = DEFAULT_COMMAND) {
  const payload = extractBridgePayload(out);
  if (payload && typeof payload === 'object') return payload;
  const stderr = out && typeof out.stderr === 'string' ? out.stderr.trim() : '';
  return {
    ok: false,
    type: 'security_layer_inventory_gate',
    command,
    error: stderr || 'security_layer_inventory_gate_kernel_bridge_failed',
    fail_closed: true
  };
}

function run(argv = []) {
  const args = normalizeArgs(argv);
  const command = String(args[0] || DEFAULT_COMMAND).trim().toLowerCase() || DEFAULT_COMMAND;
  const out = bridge.run(args.length ? args : [DEFAULT_COMMAND]);
  return normalizePayload(out, command);
}

function bridgeStatusCode(out) {
  const parsed = Number(out && out.status);
  return Number.isFinite(parsed) ? parsed : 1;
}

function emitBridgeOutput(out, payload) {
  if (out && out.stdout) process.stdout.write(String(out.stdout));
  else if (payload && typeof payload === 'object') process.stdout.write(`${JSON.stringify(payload)}\n`);
  if (out && out.stderr) process.stderr.write(String(out.stderr));
}

function runCli(argv = process.argv.slice(2)) {
  const args = normalizeArgs(argv);
  const command = String(args[0] || DEFAULT_COMMAND).trim().toLowerCase() || DEFAULT_COMMAND;
  const out = bridge.run(args.length ? args : [DEFAULT_COMMAND]);
  const payload = normalizePayload(out, command);
  emitBridgeOutput(out, payload);
  return bridgeStatusCode(out);
}

module.exports = {
  BRIDGE_PATH,
  KERNEL_ID,
  DEFAULT_COMMAND,
  normalizeArgs,
  run,
  runCli,
};

if (require.main === module) {
  process.exit(runCli(process.argv.slice(2)));
}
