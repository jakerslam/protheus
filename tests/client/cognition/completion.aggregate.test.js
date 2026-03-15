#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../..');
const completion = require(path.join(ROOT, 'client/cognition/orchestration/completion.ts'));
const taskgroup = require(path.join(ROOT, 'client/cognition/orchestration/taskgroup.ts'));

function main() {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'orchestration-completion-aggregate-'));
  const created = taskgroup.ensureTaskGroup({
    task_group_id: 'aggregate-20260315000000-abc123',
    task_type: 'audit',
    coordinator_session: 'session-c',
    agent_count: 3
  }, { rootDir });
  assert.strictEqual(created.ok, true);

  const batched = completion.trackBatchCompletion(created.task_group.task_group_id, [
    { agent_id: 'agent-1', status: 'done' },
    { agent_id: 'agent-2', status: 'timeout', details: { partial_results_count: 2 } },
    { agent_id: 'agent-3', status: 'done' }
  ], { rootDir });

  assert.strictEqual(batched.ok, true);
  assert.strictEqual(batched.summary.complete, true);
  assert.strictEqual(batched.summary.timeout_count, 1);
  assert.strictEqual(batched.summary.partial_count, 1);
  assert.notStrictEqual(batched.notification, null);

  console.log(JSON.stringify({ ok: true, type: 'orchestration_completion_aggregate_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
