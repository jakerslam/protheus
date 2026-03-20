#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.resolve(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const TARGET = path.resolve(ROOT, 'client/runtime/systems/ui/infring_dashboard.ts');
const SNAPSHOT_PATH = path.resolve(
  ROOT,
  'client/runtime/local/state/ui/infring_dashboard/latest_snapshot.json'
);

function runSnapshot() {
  return spawnSync(process.execPath, [ENTRYPOINT, TARGET, 'snapshot', '--pretty=0'], {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: 'pipe',
    env: process.env,
    maxBuffer: 16 * 1024 * 1024,
  });
}

function parseJson(text) {
  const raw = String(text || '').trim();
  assert(raw.length > 0, 'snapshot output should not be empty');
  return JSON.parse(raw);
}

const proc = runSnapshot();
assert.strictEqual(proc.status, 0, `snapshot command failed: ${proc.stderr || proc.stdout}`);

const payload = parseJson(proc.stdout);
assert.strictEqual(payload.type, 'infring_dashboard_snapshot');
assert.strictEqual(payload.metadata.authority, 'rust_core_lanes');
assert.ok(payload.health && typeof payload.health === 'object', 'health payload missing');
assert.ok(payload.app && typeof payload.app === 'object', 'app payload missing');
assert.ok(payload.collab && typeof payload.collab === 'object', 'collab payload missing');
assert.ok(payload.skills && typeof payload.skills === 'object', 'skills payload missing');
assert.ok(Array.isArray(payload.receipts.recent), 'receipts.recent should be an array');
assert.ok(Array.isArray(payload.logs.recent), 'logs.recent should be an array');
assert.ok(Array.isArray(payload.memory.entries), 'memory.entries should be an array');
assert.ok(typeof payload.receipt_hash === 'string' && payload.receipt_hash.length > 20);
assert.ok(fs.existsSync(SNAPSHOT_PATH), 'snapshot receipt file missing');

const onDisk = JSON.parse(fs.readFileSync(SNAPSHOT_PATH, 'utf8'));
assert.strictEqual(onDisk.type, 'infring_dashboard_snapshot');
assert.strictEqual(onDisk.receipt_hash, payload.receipt_hash);
