#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../..');

function resetModule(modulePath) {
  delete require.cache[require.resolve(modulePath)];
  return require(modulePath);
}

function main() {
  process.env.PROTHEUS_OPS_USE_PREBUILT = '0';
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = '120000';

  const mod = resetModule(path.join(ROOT, 'client/lib/autonomy_receipt_schema.ts'));
  const record = mod.toSuccessCriteriaRecord({}, { required: true, min_count: 2 });
  assert.equal(record.required, true);
  assert.equal(record.min_count, 2);

  const verification = mod.withSuccessCriteriaVerification(
    { checks: [{ name: 'other_check', pass: true }], failed: [], outcome: 'shipped' },
    { required: true, passed: false, primary_failure: 'success_criteria_missing' },
    { enforceNoChangeOnFailure: true }
  );
  assert.equal(verification.passed, false);
  assert.equal(verification.outcome, 'no_change');
  assert.ok(verification.failed.includes('success_criteria_met'));

  const normalized = mod.normalizeAutonomyReceiptForWrite({
    intent: { success_criteria_policy: { required: true, min_count: 1 } },
    verification: { checks: [], failed: [] }
  });
  assert.equal(normalized.verification.success_criteria.synthesized, true);
  assert.equal(normalized.verification.primary_failure_taxonomy, 'success_criteria_failed');

  const criteria = mod.successCriteriaFromReceipt(normalized);
  assert.equal(criteria.required, true);
  console.log(JSON.stringify({ ok: true, type: 'autonomy_receipt_schema_rust_bridge_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
