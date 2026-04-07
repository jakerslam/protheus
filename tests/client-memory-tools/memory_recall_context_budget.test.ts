#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../..');
const policyValidator = require(path.join(
  ROOT,
  'client/runtime/systems/memory/policy_validator.ts'
));

function main() {
  // V6-MEMORY-015: burn SLO must fail closed at the client boundary.
  const burnViolation = policyValidator.validateMemoryPolicy([
    'query-index',
    '--session-id=session_budget_test',
    '--burn-threshold=500',
    '--top=3'
  ]);
  assert.strictEqual(burnViolation.ok, false);
  assert.strictEqual(burnViolation.reason_code, 'burn_slo_threshold_exceeded');

  // V6-MEMORY-016: recall budget caps must reject oversized requests.
  const budgetViolation = policyValidator.validateMemoryPolicy([
    'query-index',
    '--session-id=session_budget_test',
    '--top=999',
    '--max-files=50',
    '--expand-lines=1000'
  ]);
  assert.strictEqual(budgetViolation.ok, false);
  assert.strictEqual(budgetViolation.reason_code, 'recall_budget_exceeded');

  const budgetWithinLimits = policyValidator.validateMemoryPolicy([
    'query-index',
    '--session-id=session_budget_test',
    '--top=5',
    '--max-files=2',
    '--expand-lines=80'
  ]);
  assert.strictEqual(budgetWithinLimits.ok, true);

  console.log(
    JSON.stringify({
      ok: true,
      type: 'memory_recall_context_budget_test'
    })
  );
}

if (require.main === module) {
  main();
}

module.exports = { main };
