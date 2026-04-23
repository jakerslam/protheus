#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.join(ROOT, 'client', 'runtime', 'lib', 'ts_entrypoint.ts');
const OPS = path.join(ROOT, 'client', 'runtime', 'systems', 'ops', 'run_infring_ops.ts');
const POLICY = path.join(ROOT, 'client', 'runtime', 'config', 'swarm_verification_mode_policy.json');

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
      INFRING_OPS_LOCAL_TIMEOUT_MS: '240000',
      INFRING_OPS_LOCAL_TIMEOUT_MS: '240000',
      INFRING_OPS_USE_PREBUILT: '0',
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
  const policy = JSON.parse(fs.readFileSync(POLICY, 'utf8'));
  const status = runOps(['verity-plane', 'status', '--json']);
  assert.equal(status.status, 0, `verity-plane status failed\n${status.stderr}`);
  assert(status.payload, 'expected verity status payload');
  assert.equal(status.payload.type, 'verity_plane_status');
  assert(
    ['normal', 'verification_mode', 'judicial_lock'].includes(status.payload.verification_mode),
    `unexpected verification mode ${String(status.payload.verification_mode)}`
  );

  const record = runOps([
    'verity-plane',
    'record',
    '--operation=swarm-verification-smoke',
    '--metadata-json={"source":"swarm_verification_mode_test","quorum":3}',
    '--json',
  ]);
  assert.equal(record.status, 0, `verity-plane record failed\n${record.stderr}`);
  assert(record.payload, 'expected verity record payload');
  assert.equal(record.payload.type, 'verity_record_event');
  assert(
    ['normal', 'verification_mode', 'judicial_lock'].includes(record.payload.verification_mode),
    `unexpected record verification mode ${String(record.payload.verification_mode)}`
  );
  assert(record.payload.receipt && record.payload.receipt.receipt_hash, 'expected verity receipt hash');

  assert.equal(policy.enabled, true);
  assert.equal(policy.quorum.min_votes, 3);
  assert.equal(policy.quorum.min_agreement_ratio, 0.67);
  assert.equal(policy.budget.max_tokens_per_verification, 12000);
  assert.equal(typeof policy.outputs.latest_path, 'string');
  assert.equal(typeof policy.outputs.history_path, 'string');

  console.log(JSON.stringify({ ok: true, type: 'swarm_verification_mode_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
