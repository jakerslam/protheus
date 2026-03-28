#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const bridge = createOpsLaneBridge(
  __dirname,
  'generate_coverage_badge',
  'coverage-badge-kernel',
  { preferLocalCore: true }
);

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
    type: 'coverage_merge_summary',
    error: stderr || 'coverage_badge_kernel_bridge_failed',
  };
}

function runKernel(args = []) {
  const passArgs = (Array.isArray(args) ? args : [])
    .map((token) => String(token || '').trim())
    .filter(Boolean);
  return bridge.run(passArgs);
}

function run(argv = process.argv.slice(2)) {
  const args = Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
  const first = args[0] || '';
  const command = first && !first.startsWith('--') ? first.toLowerCase() : 'run';
  const rest = command === 'run' && first.startsWith('--') ? args : args.slice(1);
  const normalizedCommand = command === 'run' ? 'run' : command;
  const out = runKernel([normalizedCommand, ...rest]);

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

module.exports = { run };
