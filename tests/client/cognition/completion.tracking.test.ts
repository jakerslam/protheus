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
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'orchestration-completion-track-'));
  const created = taskgroup.ensureTaskGroup({
    task_group_id: 'track-20260315000000-abc123',
    task_type: 'audit',
    coordinator_session: 'session-a',
    agent_count: 2
  }, { rootDir });
  assert.strictEqual(created.ok, true);

  const tracked = completion.trackAgentCompletion(created.task_group.task_group_id, {
    agent_id: 'agent-1',
    status: 'running'
  }, { rootDir });

  assert.strictEqual(tracked.ok, true);
  assert.strictEqual(tracked.summary.running_count, 1);
  assert.strictEqual(tracked.summary.complete, false);
  assert.strictEqual(tracked.notification, null);

  console.log(JSON.stringify({ ok: true, type: 'orchestration_completion_tracking_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
