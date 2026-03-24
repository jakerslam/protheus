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
  // V6-MEMORY-018: stale-index bypass should be blocked at client boundary.
  const staleBypass = policyValidator.validateMemoryPolicy([
    'query-index',
    '--session-id=session_freshness_test',
    '--allow-stale=1'
  ]);
  assert.strictEqual(staleBypass.ok, false);
  assert.strictEqual(staleBypass.reason_code, 'stale_override_forbidden');

  const noStaleBypass = policyValidator.validateMemoryPolicy([
    'query-index',
    '--session-id=session_freshness_test'
  ]);
  assert.strictEqual(noStaleBypass.ok, true);

  console.log(
    JSON.stringify({
      ok: true,
      type: 'memory_index_freshness_gate_test'
    })
  );
}

if (require.main === module) {
  main();
}

module.exports = { main };
