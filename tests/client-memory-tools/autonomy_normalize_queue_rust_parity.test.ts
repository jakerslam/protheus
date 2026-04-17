#!/usr/bin/env node
'use strict';

const path = require('path');
const { spawnSync } = require('child_process');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

const target = path.join(__dirname, 'autonomy_normalize_queue_pressure_rust_parity.test.ts');
const out = spawnSync(process.execPath, [target], { encoding: 'utf8' });
if (out.stdout) process.stdout.write(out.stdout);
if (out.stderr) process.stderr.write(out.stderr);
if (out.status !== 0) {
  console.error('autonomy_normalize_queue_rust_parity.test.ts: FAIL delegated to autonomy_normalize_queue_pressure_rust_parity.test.ts');
  process.exit(out.status || 1);
}
const envelope = { ok: true, status: 'ok', stdout: out.stdout || '', stderr: out.stderr || '' };
assertNoPlaceholderOrPromptLeak(envelope, 'autonomy_normalize_queue_rust_parity_test');
assertStableToolingEnvelope(envelope, 'autonomy_normalize_queue_rust_parity_test');
console.log('autonomy_normalize_queue_rust_parity.test.ts: OK');
