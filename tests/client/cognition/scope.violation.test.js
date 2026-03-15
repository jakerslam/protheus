#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../..');
const scope = require(path.join(ROOT, 'client/cognition/orchestration/scope.ts'));

function main() {
  const classified = scope.classifyFindingsByScope(
    [
      {
        item_id: 'V6-SEC-010',
        location: 'core/layer0/ops/src/security_plane.rs:12'
      },
      {
        item_id: 'V6-MEMORY-013',
        location: 'client/runtime/systems/memory/policy_validator.ts:22'
      }
    ],
    {
      scope_id: 'security-only',
      series: ['V6-SEC'],
      paths: ['core/layer0/ops/*']
    },
    'agent-1'
  );

  assert.strictEqual(classified.ok, true);
  assert.strictEqual(classified.in_scope.length, 1);
  assert.strictEqual(classified.out_of_scope.length, 1);
  assert.strictEqual(classified.violations.length, 1);
  assert.strictEqual(classified.violations[0].reason_code, 'out_of_scope_finding');
  assert.strictEqual(classified.violations[0].agent_id, 'agent-1');

  console.log(JSON.stringify({ ok: true, type: 'orchestration_scope_violation_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
