#!/usr/bin/env node
'use strict';

const path = require('path');
const assert = require('assert');
const { spawnSync } = require('child_process');

const pkgRoot = path.resolve(__dirname, '..');
const cliPath = path.join(pkgRoot, 'bin', 'protheus.ts');
const SMOKE_TIMEOUT_MS = 20000;

function sanitizeText(value, maxLen = 4000) {
  return String(value == null ? '' : value).replace(/[\u200B\u200C\u200D\u2060\uFEFF]/g, '').replace(/[\r\n\t]+/g, ' ').trim().slice(0, maxLen);
}

function run(args) {
  const out = spawnSync(process.execPath, [cliPath, ...(Array.isArray(args) ? args : [])], {
    cwd: path.resolve(pkgRoot, '..', '..'),
    encoding: 'utf8',
    timeout: SMOKE_TIMEOUT_MS,
  });
  return {
    code: Number.isFinite(out.status) ? out.status : 1,
    stdout: String(out.stdout || ''),
    stderr: String(out.stderr || ''),
    error: out.error ? sanitizeText(out.error.message || out.error, 240) : null,
  };
}

function main() {
  const help = run(['--help']);
  assert.strictEqual(help.code, 0, help.error || help.stderr || help.stdout);
  const combined = sanitizeText(help.stdout + ' ' + help.stderr, 6000);
  assert.ok(
    combined.includes('Usage') || combined.includes('protheus') || combined.includes('ok') || combined.includes('lane_id'),
    'expected help text or structured receipt from protheus wrapper'
  );
  process.stdout.write('packages/protheus-npm/scripts/smoke.ts: OK\n');
}

try {
  main();
} catch (err) {
  process.stderr.write('packages/protheus-npm/scripts/smoke.ts: FAIL: ' + err.message + '\n');
  process.exit(1);
}
