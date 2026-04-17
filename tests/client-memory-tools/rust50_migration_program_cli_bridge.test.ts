#!/usr/bin/env node
'use strict';

const assert = require('assert');
const path = require('path');
const { spawnSync } = require('child_process');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

const ROOT = path.resolve(__dirname, '..', '..');
const SCRIPT = path.join(ROOT, 'client', 'runtime', 'systems', 'ops', 'rust50_migration_program.ts');

function parseLastJson(stdout) {
  const lines = String(stdout || '').trim().split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {}
  }
  return null;
}

const proc = spawnSync('node', [SCRIPT, 'status'], {
  cwd: ROOT,
  encoding: 'utf8',
});

assert.strictEqual(proc.status, 0, proc.stderr || proc.stdout);
const payload = parseLastJson(proc.stdout);
assert(payload, 'expected JSON payload');
assert.strictEqual(payload.ok, true);
assert.strictEqual(payload.type, 'rust50_migration_program');
assert.strictEqual(payload.command, 'status');
assertNoPlaceholderOrPromptLeak(payload, 'rust50_migration_program_cli_bridge_test');
assertStableToolingEnvelope(payload, 'rust50_migration_program_cli_bridge_test');
console.log('rust50_migration_program_cli_bridge.test.ts: OK');
