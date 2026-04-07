#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const SCRIPT = path.join(ROOT, 'client', 'runtime', 'systems', 'ops', 'rust_hotpath_inventory.ts');
const LATEST = path.join(ROOT, 'local', 'state', 'ops', 'rust_hotpath_inventory', 'latest.json');

function parseLastJson(stdout) {
  const lines = String(stdout || '').trim().split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {}
  }
  return null;
}

const proc = spawnSync('node', [SCRIPT, 'run'], {
  cwd: ROOT,
  encoding: 'utf8',
});

assert.strictEqual(proc.status, 0, proc.stderr || proc.stdout);
const payload = parseLastJson(proc.stdout);
assert(payload, 'expected JSON payload');
assert.strictEqual(payload.ok, true);
assert.strictEqual(payload.type, 'rust_hotpath_inventory');
assert(payload.tracked_rs_lines > 0, 'expected rust lines');
assert(payload.tracked_ts_lines > 0, 'expected ts lines');
assert(fs.existsSync(LATEST), 'expected latest inventory artifact');
console.log('rust_hotpath_inventory_runner.test.ts: OK');
