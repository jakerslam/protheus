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
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'orchestration-completion-notify-'));
  const created = taskgroup.ensureTaskGroup({
    task_group_id: 'notify-20260315000000-abc123',
    task_type: 'audit',
    coordinator_session: 'session-b',
    agent_count: 2
  }, { rootDir });
  assert.strictEqual(created.ok, true);

  completion.trackAgentCompletion(created.task_group.task_group_id, {
    agent_id: 'agent-1',
    status: 'done'
  }, { rootDir });

  const final = completion.trackAgentCompletion(created.task_group.task_group_id, {
    agent_id: 'agent-2',
    status: 'failed'
  }, { rootDir });

  assert.strictEqual(final.ok, true);
  assert.strictEqual(final.summary.complete, true);
  assert.notStrictEqual(final.notification, null);
  assert.strictEqual(final.notification.completed_count, 1);
  assert.strictEqual(final.notification.failed_count, 1);

  console.log(JSON.stringify({ ok: true, type: 'orchestration_completion_notification_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
