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

const fromList = runAutoscale({
  mode: 'parse_lower_list',
  parse_lower_list_input: { list: [' Web ', 'FETCH', '', 'Parse'] }
});
assert.deepEqual(fromList.payload.items, ['web', 'fetch', 'parse']);

const fromCsv = runAutoscale({
  mode: 'parse_lower_list',
  parse_lower_list_input: { csv: ' Browser, Crawl ,, DIFF ' }
});
assert.deepEqual(fromCsv.payload.items, ['browser', 'crawl', 'diff']);

console.log(JSON.stringify({ ok: true, type: 'autonomy_parse_lower_list_rust_parity_test' }));
