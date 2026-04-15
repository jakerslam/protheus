#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'top50_roi_sweep', 'top50-roi-sweep-kernel');
const DEFAULT_COMMAND = 'run';
const ALLOWED_COMMANDS = new Set(['run', 'queue', 'status']);

function normalizeArgs(argv = []) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
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

function normalizePayload(out) {
  const payload = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  if (payload && typeof payload === 'object') return payload;
  const parsed = out && typeof out.stdout === 'string' ? parseLastJson(out.stdout) : null;
  if (parsed && typeof parsed === 'object') return parsed;
  const stderr = out && typeof out.stderr === 'string' ? out.stderr.trim() : '';
  return {
    ok: false,
    type: 'top50_roi_sweep',
    error: stderr || 'top50_roi_sweep_kernel_bridge_failed'
  };
}

function runKernel(command, args = []) {
  const passArgs = [
    String(command || '').trim(),
    ...normalizeArgs(args),
  ].filter(Boolean);
  return bridge.run(passArgs);
}

function buildQueue(argv = []) {
  const args = normalizeArgs(argv);
  return normalizePayload(runKernel('queue', args));
}

function run(argv = process.argv.slice(2)) {
  const args = normalizeArgs(argv);
  const command = normalizeCommand(args[0]);
  const rest =
    command === DEFAULT_COMMAND && (args[0] || '').startsWith('--') ? args : args.slice(1);
  const normalizedRest =
    command === DEFAULT_COMMAND && !ALLOWED_COMMANDS.has(String(args[0] || '').toLowerCase())
      ? args
      : rest;

  const out = runKernel(command, normalizedRest);
  const payload = normalizePayload(out);
  if (payload && typeof payload === 'object') {
    process.stdout.write(`${JSON.stringify(payload)}\n`);
  } else if (out && typeof out.stdout === 'string' && out.stdout.trim()) {
    process.stdout.write(out.stdout.endsWith('\n') ? out.stdout : `${out.stdout}\n`);
  } else {
    process.stdout.write(
      `${JSON.stringify({ ok: false, type: 'top50_roi_sweep', error: 'empty_kernel_response' })}\n`,
    );
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
  buildQueue,
  normalizeArgs,
  normalizeCommand,
  run,
};
