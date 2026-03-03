#!/usr/bin/env node
'use strict';

const assert = require('assert');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const { runBacklogAutoscalePrimitive } = require(path.join(ROOT, 'systems', 'autonomy', 'backlog_autoscale_rust_bridge.js'));

function fail(msg) {
  console.error(`❌ autonomy_backlog_autoscale_rust_bridge.test.js: ${msg}`);
  process.exit(1);
}

function ensureReleaseBinary() {
  const out = spawnSync('cargo', ['build', '--manifest-path', 'crates/execution/Cargo.toml', '--release'], {
    cwd: ROOT,
    encoding: 'utf8'
  });
  if (Number(out.status) !== 0) {
    fail(`cargo build failed: ${(out.stderr || out.stdout || '').slice(0, 300)}`);
  }
}

function getPayload(result, label) {
  if (!result || result.ok !== true || !result.payload || typeof result.payload !== 'object') {
    fail(`${label}: rust bridge invocation failed: ${JSON.stringify(result || {})}`);
  }
  if (result.payload.ok !== true || !result.payload.payload || typeof result.payload.payload !== 'object') {
    fail(`${label}: invalid bridge payload: ${JSON.stringify(result.payload || {})}`);
  }
  return result.payload.payload;
}

function main() {
  ensureReleaseBinary();

  const planInput = {
    queue_pressure: { pressure: 'critical', pending: 41, pending_ratio: 0.72 },
    min_cells: 0,
    max_cells: 4,
    current_cells: 1,
    run_interval_minutes: 10,
    idle_release_minutes: 120,
    autopause_active: false,
    last_run_minutes_ago: 25,
    last_high_pressure_minutes_ago: 4,
    trit_shadow_blocked: false
  };
  const first = getPayload(runBacklogAutoscalePrimitive('plan', planInput, { allow_cli_fallback: true }), 'plan:first');
  const second = getPayload(runBacklogAutoscalePrimitive('plan', planInput, { allow_cli_fallback: true }), 'plan:second');

  assert.strictEqual(first.action, 'scale_up');
  assert.strictEqual(Number(first.target_cells), 4);
  assert.deepStrictEqual(first, second, 'plan output should be deterministic');

  const batchInput = {
    enabled: true,
    max_batch: 6,
    daily_remaining: 4,
    pressure: 'critical',
    current_cells: 4,
    budget_blocked: true,
    trit_shadow_blocked: false
  };
  const batch = getPayload(runBacklogAutoscalePrimitive('batch_max', batchInput, { allow_cli_fallback: true }), 'batch');
  assert.strictEqual(Number(batch.max), 1);
  assert.strictEqual(batch.reason, 'budget_blocked');

  console.log('autonomy_backlog_autoscale_rust_bridge.test.js: OK');
}

try {
  main();
} catch (err) {
  fail(err && err.message ? err.message : String(err));
}
