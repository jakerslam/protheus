#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../..');
const taskgroup = require(path.join(ROOT, 'client/cognition/orchestration/taskgroup.ts'));

function main() {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'orchestration-taskgroup-query-'));
  const created = taskgroup.ensureTaskGroup({
    task_group_id: 'audit-20260315000000-abc123',
    task_type: 'audit',
    coordinator_session: 'session-main',
    agent_count: 2
  }, { rootDir });
  assert.strictEqual(created.ok, true);

  const running = taskgroup.updateAgentStatus(created.task_group.task_group_id, 'agent-1', 'running', {}, { rootDir });
  assert.strictEqual(running.ok, true);

  const finished = taskgroup.updateAgentStatus(created.task_group.task_group_id, 'agent-1', 'done', {}, { rootDir });
  assert.strictEqual(finished.ok, true);

  const timeout = taskgroup.updateAgentStatus(created.task_group.task_group_id, 'agent-2', 'timeout', {
    partial_results_count: 2
  }, { rootDir });
  assert.strictEqual(timeout.ok, true);

  const queried = taskgroup.queryTaskGroup(created.task_group.task_group_id, { rootDir });
  assert.strictEqual(queried.ok, true);
  assert.strictEqual(queried.counts.done, 1);
  assert.strictEqual(queried.counts.timeout, 1);
  assert.strictEqual(queried.task_group.status, 'completed');

  console.log(JSON.stringify({ ok: true, type: 'orchestration_taskgroup_query_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
