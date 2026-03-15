#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../..');
const taskgroup = require(path.join(ROOT, 'client/cognition/orchestration/taskgroup.ts'));

function main() {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'orchestration-taskgroup-meta-'));
  const created = taskgroup.ensureTaskGroup({
    task_type: 'compliance',
    coordinator_session: 'coordinator-session-42',
    agent_count: 1,
    agents: [{ agent_id: 'agent-alpha' }]
  }, { rootDir });

  assert.strictEqual(created.ok, true);
  assert.strictEqual(created.task_group.task_type, 'compliance');
  assert.strictEqual(created.task_group.coordinator_session, 'coordinator-session-42');
  assert.strictEqual(typeof created.task_group.created_at, 'string');
  assert.strictEqual(typeof created.task_group.updated_at, 'string');
  assert.strictEqual(created.task_group.agents[0].agent_id, 'agent-alpha');
  assert.strictEqual(Array.isArray(created.task_group.history), true);

  const savedPath = taskgroup.taskGroupPath(created.task_group.task_group_id, { rootDir });
  assert.strictEqual(fs.existsSync(savedPath), true);

  console.log(JSON.stringify({ ok: true, type: 'orchestration_taskgroup_metadata_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
