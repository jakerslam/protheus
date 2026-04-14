#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const fs = require('node:fs');
const path = require('node:path');
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
  if (payload && payload.payload && typeof payload.payload === 'object') {
    if (payload.payload.payload && typeof payload.payload.payload === 'object') {
      return payload.payload.payload;
    }
    return payload.payload;
  }
  if (payload) return payload;
  const parsed = out && typeof out.stdout === 'string' ? parseLastJson(out.stdout) : null;
  if (parsed && parsed.payload && typeof parsed.payload === 'object') {
    if (parsed.payload.payload && typeof parsed.payload.payload === 'object') {
      return parsed.payload.payload;
    }
    return parsed.payload;
  }
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

function parseArgValue(args = [], key) {
  const list = Array.isArray(args) ? args.map((token) => String(token || '')) : [];
  const inline = list.find((token) => token.startsWith(`${key}=`));
  if (inline) return inline.slice(key.length + 1).trim();
  const idx = list.findIndex((token) => token === key);
  if (idx >= 0 && idx + 1 < list.length) return String(list[idx + 1] || '').trim();
  return '';
}

function writeJsonArtifact(filePath, payload) {
  if (!filePath) return;
  const abs = path.resolve(String(filePath));
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function buildInventory(argv = []) {
  const args = Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
  return normalizePayload(runKernel('inventory', args));
}

function run(argv = process.argv.slice(2)) {
  const args = Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
  const first = args[0] || '';
  const command = first && !first.startsWith('--') ? first.toLowerCase() : 'status';
  const outJsonPath = parseArgValue(args, '--out-json') || parseArgValue(args, '--out');
  if (command !== 'run' && command !== 'status' && command !== 'inventory') {
    process.stderr.write('Usage: node client/runtime/systems/ops/rust_hotpath_inventory.ts <run|status|inventory> [--policy=<path>]\n');
    return 2;
  }
  const rest = first && !first.startsWith('--') ? args.slice(1) : args;

  const out = runKernel(command, rest);
  const payload = normalizePayload(out);
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  writeJsonArtifact(outJsonPath, payload);
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
