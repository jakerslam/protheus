#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
const bridge = createOpsLaneBridge(__dirname, 'benchmark_autonomy_gate', 'benchmark-autonomy-gate', {
  inheritStdio: false
});
const DEFAULT_COMMAND = 'run';
const ALLOWED_COMMANDS = new Set(['run', 'status']);

function normalizeArgs(argv = []) {
  return Array.isArray(argv) ? argv.map((arg) => String(arg || '').trim()).filter(Boolean) : [];
}

function normalizeCommand(raw) {
  const token = String(raw || '').trim().toLowerCase();
  if (!token || token.startsWith('--')) return DEFAULT_COMMAND;
  return ALLOWED_COMMANDS.has(token) ? token : DEFAULT_COMMAND;
}

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

function normalizePayload(out, command) {
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  if (receipt && receipt.payload && typeof receipt.payload === 'object') {
    return receipt.payload;
  }
  const parsed = parseLastJson(out && typeof out.stdout === 'string' ? out.stdout : '');
  if (parsed && typeof parsed === 'object') return parsed;
  return {
    ok: false,
    type: 'benchmark_autonomy_gate',
    command,
    error: out && out.stderr ? String(out.stderr).trim() || 'benchmark_autonomy_gate_bridge_failed' : 'benchmark_autonomy_gate_bridge_failed',
    fail_closed: true
  };
}

function run(args = []) {
  const argv = normalizeArgs(args);
  const command = normalizeCommand(argv[0]);
  const passArgs =
    command === DEFAULT_COMMAND && !ALLOWED_COMMANDS.has(String(argv[0] || '').toLowerCase())
      ? [DEFAULT_COMMAND, ...argv]
      : command === DEFAULT_COMMAND && (argv[0] || '').startsWith('--')
      ? [DEFAULT_COMMAND, ...argv]
      : argv.length
      ? argv
      : [DEFAULT_COMMAND];
  const out = bridge.run(passArgs);
  return normalizePayload(out, command);
}

module.exports = {
  normalizeArgs,
  normalizeCommand,
  run,
};

if (require.main === module) {
  const result = run(process.argv.slice(2));
  process.stdout.write(`${JSON.stringify(result)}\n`);
  if (!result.ok) process.exit(2);
}
