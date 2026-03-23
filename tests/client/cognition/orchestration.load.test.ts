#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../..');
const coordinator = require(path.join(ROOT, 'client/cognition/orchestration/coordinator.ts'));

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
    location: `client/cognition/orchestration/${index}.ts:1`,
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
  assert.strictEqual(out.scope_violation_count, 0);

  console.log(JSON.stringify({ ok: true, type: 'orchestration_load_test', agent_count: agentCount }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
