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
  // V6-MEMORY-013 index-first guard
  const v013 = policyValidator.validateMemoryPolicy([
    'query-index',
    '--session-id=session-v6-memory-regression',
    '--file=local/workspace/memory/2026-03-15.md'
  ]);
  assert.strictEqual(v013.ok, false);
  assert.strictEqual(v013.reason_code, 'direct_file_read_forbidden');

  // V6-MEMORY-014 lazy hydration guard
  const v014 = policyValidator.validateMemoryPolicy([
    'query-index',
    '--session-id=session-v6-memory-regression',
    '--bootstrap=1',
    '--lazy-hydration=0'
  ]);
  assert.strictEqual(v014.ok, false);
  assert.strictEqual(v014.reason_code, 'bootstrap_requires_lazy_hydration');

  // V6-MEMORY-015 burn SLO guard
  const v015 = policyValidator.validateMemoryPolicy([
    'query-index',
    '--session-id=session-v6-memory-regression',
    '--burn-threshold=250'
  ]);
  assert.strictEqual(v015.ok, false);
  assert.strictEqual(v015.reason_code, 'burn_slo_threshold_exceeded');

  // V6-MEMORY-016 recall budget contract
  const v016 = policyValidator.validateMemoryPolicy([
    'query-index',
    '--session-id=session-v6-memory-regression',
    '--top=80'
  ]);
  assert.strictEqual(v016.ok, false);
  assert.strictEqual(v016.reason_code, 'recall_budget_exceeded');

  // V6-MEMORY-017 ranking invariants
  const v017 = policyValidator.validateMemoryPolicy([
    'query-index',
    '--session-id=session-v6-memory-regression',
    '--scores-json=[0.8,0.91]',
    '--ids-json=["b","a"]'
  ]);
  assert.strictEqual(v017.ok, false);
  assert.strictEqual(v017.reason_code, 'ranking_not_descending');

  // V6-MEMORY-018 freshness / stale override gate
  const v018 = policyValidator.validateMemoryPolicy([
    'query-index',
    '--session-id=session-v6-memory-regression',
    '--allow-stale=1'
  ]);
  assert.strictEqual(v018.ok, false);
  assert.strictEqual(v018.reason_code, 'stale_override_forbidden');

  // V6-MEMORY-019 lensmap annotation schema
  const v019Fail = policyValidator.validateMemoryPolicy([
    'query-index',
    '--session-id=session-v6-memory-regression',
    '--lensmap-annotation-json={"node_id":"node-a","tags":[],"jots":[]}'
  ]);
  assert.strictEqual(v019Fail.ok, false);
  assert.strictEqual(v019Fail.reason_code, 'lensmap_annotation_missing_tags_or_jots');

  const v019Pass = policyValidator.validateMemoryPolicy([
    'query-index',
    '--session-id=session-v6-memory-regression',
    '--lensmap-annotation-json={"node_id":"node-a","tags":["memory"],"jots":["note"]}'
  ]);
  assert.strictEqual(v019Pass.ok, true);

  console.log(
    JSON.stringify({
      ok: true,
      type: 'v6_memory_013_019_client_regression_test'
    })
  );
}

if (require.main === module) {
  main();
}

module.exports = { main };
