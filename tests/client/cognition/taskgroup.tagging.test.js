#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../..');
const taskgroup = require(path.join(ROOT, 'client/cognition/orchestration/taskgroup.ts'));

function main() {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'orchestration-taskgroup-tagging-'));
  const created = taskgroup.ensureTaskGroup({
    task_type: 'srs-audit',
    coordinator_session: 'session-coordinator',
    agent_count: 3
  }, { rootDir });

  assert.strictEqual(created.ok, true);
  assert.strictEqual(created.created, true);
  assert.strictEqual(created.task_group.agent_count, 3);
  assert.strictEqual(created.task_group.agents.length, 3);
  assert.match(created.task_group.task_group_id, /^srs-audit-\d{14}-[a-z0-9._-]+$/);

  console.log(JSON.stringify({ ok: true, type: 'orchestration_taskgroup_tagging_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
