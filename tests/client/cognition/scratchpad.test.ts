#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../..');
const scratchpad = require(path.join(ROOT, 'client/cognition/orchestration/scratchpad.ts'));

function main() {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'orchestration-scratchpad-'));
  const taskId = 'audit-task-001';

  const write = scratchpad.writeScratchpad(taskId, {
    progress: { processed: 1, total: 3 }
  }, { rootDir });
  assert.strictEqual(write.ok, true);
  assert.strictEqual(write.scratchpad.schema_version, 'scratchpad/v1');

  const appendFinding = scratchpad.appendFinding(taskId, {
    audit_id: 'audit-001',
    item_id: 'item-001',
    severity: 'medium',
    status: 'open',
    location: '/tmp/a.ts:1',
    evidence: [{ type: 'receipt', value: 'receipt-1' }],
    timestamp: new Date().toISOString()
  }, { rootDir });
  assert.strictEqual(appendFinding.ok, true);
  assert.strictEqual(appendFinding.finding_count, 1);

  const appendCheckpoint = scratchpad.appendCheckpoint(taskId, {
    reason: 'manual',
    processed_count: 1
  }, { rootDir });
  assert.strictEqual(appendCheckpoint.ok, true);
  assert.strictEqual(appendCheckpoint.checkpoint_count, 1);

  const loaded = scratchpad.loadScratchpad(taskId, { rootDir });
  assert.strictEqual(loaded.exists, true);
  assert.strictEqual(loaded.scratchpad.findings.length, 1);
  assert.strictEqual(loaded.scratchpad.checkpoints.length, 1);

  const cleanup = scratchpad.cleanupScratchpad(taskId, { rootDir });
  assert.strictEqual(cleanup.ok, true);
  assert.strictEqual(cleanup.removed, true);

  console.log(JSON.stringify({ ok: true, type: 'orchestration_scratchpad_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
