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
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'orchestration-integration-'));
  const out = coordinator.runCoordinator({
    task_id: 'integration-audit-task',
    task_type: 'integration-audit',
    coordinator_session: 'session-integration',
    agent_count: 2,
    items: ['V6-SEC-010', 'V6-MEMORY-013', 'REQ-38-004'],
    scopes: [
      { scope_id: 'scope-sec', series: ['V6-SEC'], paths: ['core/layer0/*'] },
      { scope_id: 'scope-memory', series: ['V6-MEMORY', 'REQ-38'], paths: ['client/*'] }
    ],
    findings: [
      {
        audit_id: 'integration-audit',
        agent_id: 'agent-1',
        item_id: 'V6-SEC-010',
        severity: 'high',
        status: 'open',
        location: 'core/layer0/ops/src/security_plane.rs:10',
        evidence: [{ type: 'receipt', value: 'sec-1' }],
        timestamp: new Date('2026-03-15T00:00:00Z').toISOString()
      },
      {
        audit_id: 'integration-audit',
        agent_id: 'agent-1',
        item_id: 'V6-MEMORY-013',
        severity: 'medium',
        status: 'open',
        location: 'client/runtime/systems/memory/policy_validator.ts:10',
        evidence: [{ type: 'receipt', value: 'out-of-scope' }],
        timestamp: new Date('2026-03-15T00:00:01Z').toISOString()
      },
      {
        audit_id: 'integration-audit',
        agent_id: 'agent-2',
        item_id: 'REQ-38-004',
        severity: 'low',
        status: 'open',
        location: 'client/cognition/orchestration/scope.ts:20',
        evidence: [{ type: 'receipt', value: 'req38' }],
        timestamp: new Date('2026-03-15T00:00:02Z').toISOString()
      }
    ],
    root_dir: rootDir
  });

  assert.strictEqual(out.ok, true);
  assert.strictEqual(out.findings_total, 3);
  assert.strictEqual(out.findings_in_scope, 2);
  assert.strictEqual(out.scope_violation_count, 1);
  assert.strictEqual(out.completion_summary.complete, true);
  assert.notStrictEqual(out.notification, null);

  const group = taskgroup.queryTaskGroup(out.task_group_id, { rootDir });
  assert.strictEqual(group.ok, true);
  assert.strictEqual(group.counts.done, 2);
  assert.strictEqual(group.task_group.status, 'done');

  console.log(JSON.stringify({ ok: true, type: 'orchestration_integration_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
