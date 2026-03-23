#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../..');
const scratchpad = require(path.join(ROOT, 'client/cognition/orchestration/scratchpad.ts'));
const checkpoint = require(path.join(ROOT, 'client/cognition/orchestration/checkpoint.ts'));

function main() {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'orchestration-checkpoint-'));
  const taskId = 'audit-task-checkpoint';
  scratchpad.writeScratchpad(taskId, { progress: { processed: 0, total: 20 } }, { rootDir });

  const firstTick = checkpoint.maybeCheckpoint(taskId, {
    processed_count: 5,
    total_count: 20,
    now_ms: 1000
  }, { rootDir });
  assert.strictEqual(firstTick.ok, true);
  assert.strictEqual(firstTick.checkpoint_written, true);

  const secondTick = checkpoint.maybeCheckpoint(taskId, {
    processed_count: 9,
    total_count: 20,
    now_ms: 1050
  }, { rootDir });
  assert.strictEqual(secondTick.ok, true);
  assert.strictEqual(secondTick.checkpoint_written, false);

  const thirdTick = checkpoint.maybeCheckpoint(taskId, {
    processed_count: 15,
    total_count: 20,
    now_ms: 3000
  }, { rootDir });
  assert.strictEqual(thirdTick.ok, true);
  assert.strictEqual(thirdTick.checkpoint_written, true);

  const timeout = checkpoint.handleTimeout(taskId, {
    processed_count: 16,
    total_count: 20,
    partial_results: [{ item_id: 'item-9' }],
    retry_count: 0,
    now_ms: 3500
  }, { rootDir });
  assert.strictEqual(timeout.ok, true);
  assert.strictEqual(Array.isArray(timeout.partial_results), true);
  assert.strictEqual(timeout.partial_results.length, 1);
  assert.strictEqual(typeof timeout.checkpoint_path, 'string');
  assert.strictEqual(timeout.retry_allowed, true);

  console.log(JSON.stringify({ ok: true, type: 'orchestration_checkpoint_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
