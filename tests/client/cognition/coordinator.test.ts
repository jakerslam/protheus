#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../..');
const coordinator = require(path.join(ROOT, 'client/cognition/orchestration/coordinator.ts'));
const scratchpad = require(path.join(ROOT, 'client/cognition/orchestration/scratchpad.ts'));

function main() {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'orchestration-coordinator-'));
  const result = coordinator.runCoordinator({
    task_id: 'audit-task-coordinator',
    audit_id: 'audit-001',
    task_type: 'srs-audit',
    coordinator_session: 'session-main',
    agent_count: 2,
    items: ['item-1', 'item-2', 'item-3', 'item-4'],
    scopes: [
      { scope_id: 'scope-security', series: ['ITEM-2'], paths: ['/tmp/a.ts*'] },
      { scope_id: 'scope-memory', series: ['ITEM-3'], paths: ['/tmp/b.ts*'] }
    ],
    findings: [
      {
        audit_id: 'audit-001',
        agent_id: 'agent-1',
        item_id: 'item-2',
        severity: 'low',
        status: 'open',
        location: '/tmp/a.ts:1',
        evidence: [{ type: 'receipt', value: 'a' }],
        timestamp: new Date('2026-03-15T00:00:00Z').toISOString()
      },
      {
        audit_id: 'audit-001',
        agent_id: 'agent-1',
        item_id: 'item-2',
        severity: 'critical',
        status: 'confirmed',
        location: '/tmp/a.ts:1',
        evidence: [{ type: 'receipt', value: 'b' }],
        timestamp: new Date('2026-03-15T00:01:00Z').toISOString()
      },
      {
        audit_id: 'audit-001',
        agent_id: 'agent-2',
        item_id: 'item-3',
        severity: 'medium',
        status: 'open',
        location: '/tmp/b.ts:2',
        evidence: [{ type: 'receipt', value: 'c' }],
        timestamp: new Date('2026-03-15T00:02:00Z').toISOString()
      }
    ],
    root_dir: rootDir
  });

  assert.strictEqual(result.ok, true);
  assert.strictEqual(result.partition_count, 2);
  assert.strictEqual(result.findings_total, 3);
  assert.strictEqual(result.findings_merged, 2);
  assert.strictEqual(result.report.findings[0].item_id, 'item-2');
  assert.strictEqual(result.report.findings[0].severity, 'critical');
  assert.strictEqual(result.report.findings[0].status, 'confirmed');
  assert.strictEqual(result.report.findings[0].evidence.length, 2);
  assert.strictEqual(result.scope_violation_count, 0);
  assert.strictEqual(result.completion_summary.complete, true);
  assert.notStrictEqual(result.notification, null);
  assert.match(result.task_group_id, /^srs-audit-\d{14}-[a-z0-9._-]+$/);

  const loaded = scratchpad.loadScratchpad('audit-task-coordinator', { rootDir });
  assert.strictEqual(loaded.exists, true);
  assert.strictEqual(loaded.scratchpad.findings.length, 2);
  assert.strictEqual(loaded.scratchpad.progress.total, 4);
  assert.strictEqual(loaded.scratchpad.progress.processed, 2);

  console.log(JSON.stringify({ ok: true, type: 'orchestration_coordinator_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
