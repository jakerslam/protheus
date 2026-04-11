#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../..');
const coordinator = require(path.join(ROOT, 'client/cognition/orchestration/coordinator.ts'));
const taskgroup = require(path.join(ROOT, 'client/cognition/orchestration/taskgroup.ts'));

function main() {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'orchestration-load-'));
  const agentCount = 20;
  const items = Array.from({ length: 40 }, (_, idx) => `REQ-38-${String(idx + 1).padStart(3, '0')}`);
  const findings = items.map((itemId, index) => ({
    audit_id: 'load-audit',
    agent_id: `agent-${(index % agentCount) + 1}`,
    item_id: itemId,
    severity: 'low',
    status: 'open',
    location: `core/layer0/ops/src/orchestration_parts/080-invoke.rs:${index + 1}`,
    evidence: [{ type: 'receipt', value: `receipt-${index}` }],
    timestamp: new Date('2026-03-15T00:00:00Z').toISOString()
  }));

  const out = coordinator.runCoordinator({
    task_id: 'load-test-task',
    task_type: 'load-audit',
    coordinator_session: 'load-session',
    agent_count: agentCount,
    items,
    findings,
    root_dir: rootDir
  });

  assert.strictEqual(out.ok, true);
  assert.strictEqual(out.partition_count, agentCount);
  assert.strictEqual(out.findings_total, findings.length);
  assert.strictEqual(out.findings_merged, findings.length);
  assert.strictEqual(out.completion_summary.complete, true);
  assert.strictEqual(out.completion_summary.total_count, agentCount);
  assert.strictEqual(out.completion_summary.partial_count, agentCount);
  assert.strictEqual(out.scope_violation_count, 0);
  assert.strictEqual(out.notification.total_count, agentCount);
  assert.strictEqual(out.partitions.every((row) => row.items.length === 2), true);
  assert.strictEqual(out.report.findings[0].location, 'core/layer0/ops/src/orchestration_parts/080-invoke.rs:1');
  assert.strictEqual(fs.existsSync(out.checkpoint.checkpoint_path), true);

  const group = taskgroup.queryTaskGroup(out.task_group_id, { rootDir });
  assert.strictEqual(group.ok, true);
  assert.strictEqual(group.counts.done, agentCount);
  assert.strictEqual(group.task_group.history.length, agentCount);
  assert.strictEqual(
    group.task_group.agents.every((row) => row.details.partial_results_count === 2),
    true,
  );
  assert.strictEqual(
    group.task_group.agents.every((row) => row.details.processed_count === 2),
    true,
  );

  console.log(JSON.stringify({ ok: true, type: 'orchestration_load_test', agent_count: agentCount }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
