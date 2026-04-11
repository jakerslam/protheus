#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.join(ROOT, 'client', 'runtime', 'lib', 'ts_entrypoint.ts');
const OPS = path.join(ROOT, 'client', 'runtime', 'systems', 'ops', 'run_protheus_ops.ts');

function parseLastJson(stdout) {
  const text = String(stdout || '').trim();
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {}
  let depth = 0;
  let start = -1;
  let inString = false;
  let escaped = false;
  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (inString) {
      if (escaped) escaped = false;
      else if (ch === '\\') escaped = true;
      else if (ch === '"') inString = false;
      continue;
    }
    if (ch === '"') {
      inString = true;
      continue;
    }
    if (ch === '{') {
      if (depth === 0) start = i;
      depth += 1;
      continue;
    }
    if (ch === '}') {
      depth -= 1;
      if (depth === 0 && start >= 0) {
        try {
          return JSON.parse(text.slice(start, i + 1));
        } catch {}
      }
    }
  }
  const lines = text
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    if (!lines[i].startsWith('{')) continue;
    try {
      return JSON.parse(lines[i]);
    } catch {}
  }
  return null;
}

function runOps(args) {
  const out = spawnSync(process.execPath, [ENTRYPOINT, OPS].concat(args), {
    cwd: ROOT,
    encoding: 'utf8',
    env: {
      ...process.env,
      PROTHEUS_OPS_LOCAL_TIMEOUT_MS: '240000',
      INFRING_OPS_LOCAL_TIMEOUT_MS: '240000',
      PROTHEUS_OPS_USE_PREBUILT: '0',
      INFRING_OPS_USE_PREBUILT: '0',
    },
  });
  return {
    status: Number.isFinite(Number(out.status)) ? Number(out.status) : 1,
    stdout: String(out.stdout || ''),
    stderr: String(out.stderr || ''),
    payload: parseLastJson(out.stdout),
  };
}

function main() {
  const status = runOps(['rsi-ignition', 'status', '--json']);
  assert.equal(status.status, 0, `rsi-ignition status failed\n${status.stderr}`);
  assert(status.payload, 'expected rsi-ignition status payload');
  assert.equal(status.payload.type, 'rsi_ignition_status');
  assert.equal(status.payload.ok, true);
  assert.equal(typeof status.payload.loop_state?.swarm?.nodes, 'number');

  const swarm = runOps(['rsi-ignition', 'swarm', '--apply=0', '--json']);
  assert.notEqual(swarm.status, 0, 'expected rsi-ignition swarm to fail closed without explicit directive allowance');
  assert(swarm.payload, 'expected rsi-ignition swarm payload');
  assert.equal(swarm.payload.type, 'rsi_ignition_swarm');
  assert.equal(swarm.payload.ok, false);
  assert.equal(swarm.payload.gate_ok, false);
  assert.equal(typeof swarm.payload.nodes, 'number');
  assert.equal(typeof swarm.payload.share_rate, 'number');
  assert(
    Array.isArray(swarm.payload.claim_evidence)
      && swarm.payload.claim_evidence.some((row) => row.id === 'V8-RSI-IGNITION-003'),
    'expected rsi swarm payload to surface claim evidence'
  );

  console.log(JSON.stringify({ ok: true, type: 'rsi_swarm_spawn_bridge_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
