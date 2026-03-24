#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const SCRIPT = path.join(ROOT, 'tests', 'tooling', 'scripts', 'ci', 'srs_full_regression.mjs');

function parseLastJson(stdout) {
  const whole = String(stdout || '').trim();
  if (whole) {
    try {
      return JSON.parse(whole);
    } catch {}
  }
  const lines = whole.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {}
  }
  return null;
}

const rgPath = spawnSync('which', ['rg'], { encoding: 'utf8' });
assert.strictEqual(rgPath.status, 0, 'expected rg in PATH');
const realRg = String(rgPath.stdout || '').trim();
assert(realRg, 'expected real rg path');

const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'srs-full-regression-retry-'));
const counterPath = path.join(tempRoot, 'rg-counter.txt');
const wrapperPath = path.join(tempRoot, 'rg');

fs.writeFileSync(
  wrapperPath,
  `#!/bin/sh
COUNTER_FILE="${counterPath}"
COUNT=0
if [ -f "$COUNTER_FILE" ]; then
  COUNT=$(cat "$COUNTER_FILE")
fi
COUNT=$((COUNT + 1))
printf '%s' "$COUNT" > "$COUNTER_FILE"
if [ "$COUNT" -le 3 ]; then
  exit 1
fi
exec "${realRg}" "$@"
`,
  'utf8',
);
fs.chmodSync(wrapperPath, 0o755);

const proc = spawnSync('node', [SCRIPT], {
  cwd: ROOT,
  encoding: 'utf8',
  env: {
    ...process.env,
    PATH: `${tempRoot}:${process.env.PATH}`,
  },
  maxBuffer: 1024 * 1024 * 32,
});

assert.strictEqual(proc.status, 0, proc.stderr || proc.stdout);
const payload = parseLastJson(proc.stdout);
assert(payload, 'expected JSON payload');
assert.strictEqual(payload.ok, true);
assert.strictEqual(payload.type, 'srs_full_regression');
assert.strictEqual(payload.summary.regression.fail, 0, 'expected retry to recover from transient evidence collapse');
assert.strictEqual(payload.summary.doneWithoutNonBacklogEvidence, 0);
assert.strictEqual(payload.summary.doneWithoutCodeEvidence, 0);
assert(payload.summary.retry, 'expected retry metadata');
assert.strictEqual(payload.summary.retry.attempted, true, 'expected retry to be attempted');
assert.strictEqual(payload.summary.retry.used_second, true, 'expected second pass to replace broken first pass');
const calls = Number(fs.readFileSync(counterPath, 'utf8'));
assert(calls >= 6, `expected at least 6 rg calls across retry, got ${calls}`);

console.log('srs_full_regression_retry.test.ts: OK');
