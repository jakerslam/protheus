#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const MANIFEST = path.join(ROOT, 'core', 'layer2', 'execution', 'Cargo.toml');

function runAutoscale(request) {
  const payload = Buffer.from(JSON.stringify(request), 'utf8').toString('base64');
  const proc = spawnSync('cargo', ['run', '--quiet', '--manifest-path', MANIFEST, '--', 'autoscale', `--payload-base64=${payload}`], {
    cwd: ROOT,
    encoding: 'utf8'
  });
  assert.equal(proc.status, 0, proc.stderr || proc.stdout);
  return JSON.parse(proc.stdout.trim());
}

const valid = runAutoscale({
  mode: 'parse_iso_ts',
  parse_iso_ts_input: { ts: '2026-04-11T12:34:56Z' }
});
assert.equal(typeof valid.payload.timestamp_ms, 'number');
assert.equal(valid.payload.timestamp_ms > 0, true);

const invalid = runAutoscale({
  mode: 'parse_iso_ts',
  parse_iso_ts_input: { ts: 'definitely-not-a-timestamp' }
});
assert.equal(invalid.payload.timestamp_ms, null);

console.log(JSON.stringify({ ok: true, type: 'autonomy_parse_iso_ts_rust_parity_test' }));
