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

const out = runAutoscale({
  mode: 'parse_json_objects_from_text',
  parse_json_objects_from_text_input: {
    text: 'noise\n{"a":1}\n[]\n{"b":2}\n{"c":3}',
    max_objects: 2
  }
});
assert.deepEqual(out.payload.objects, [{ a: 1 }, { b: 2 }]);

console.log(JSON.stringify({ ok: true, type: 'autonomy_parse_json_objects_from_text_rust_parity_test' }));
