#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../..');
const checkpoint = require(path.join(ROOT, 'client/cognition/orchestration/checkpoint.ts'));
const scratchpad = require(path.join(ROOT, 'client/cognition/orchestration/scratchpad.ts'));
const partial = require(path.join(ROOT, 'client/cognition/orchestration/partial.ts'));

function main() {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'orchestration-partial-checkpoint-'));
  const taskId = 'partial-checkpoint-task';

  scratchpad.writeScratchpad(taskId, { progress: { processed: 0, total: 4 } }, { rootDir });
  checkpoint.handleTimeout(taskId, {
    processed_count: 2,
    total_count: 4,
    partial_results: [{ item_id: 'V6-MEMORY-013' }],
    retry_count: 0,
    now_ms: Date.now()
  }, { rootDir });

  const out = partial.retrievePartialResults({
    task_id: taskId,
    session_history: [],
    root_dir: rootDir
  });

  assert.strictEqual(out.ok, true);
  assert.strictEqual(out.source, 'checkpoint');
  assert.strictEqual(out.items_completed, 2);
  assert.strictEqual(out.findings_sofar.length, 1);
  assert.strictEqual(typeof out.checkpoint_path, 'string');

  console.log(JSON.stringify({ ok: true, type: 'orchestration_partial_checkpoint_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
