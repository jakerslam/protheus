#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../..');
const scope = require(path.join(ROOT, 'client/cognition/orchestration/scope.ts'));

function main() {
  const valid = scope.detectScopeOverlaps([
    { scope_id: 'sec-a', series: ['V6-SEC'], paths: ['core/layer0/ops/*'] },
    { scope_id: 'mem-b', series: ['V6-MEMORY'], paths: ['client/runtime/systems/memory/*'] }
  ]);
  assert.strictEqual(valid.ok, true);
  assert.strictEqual(valid.normalized_scopes.length, 2);
  assert.strictEqual(valid.overlaps.length, 0);

  const invalid = scope.detectScopeOverlaps([
    { scope_id: 'bad-empty' }
  ]);
  assert.strictEqual(invalid.ok, false);
  assert.strictEqual(invalid.reason_code, 'scope_missing_series_and_paths');

  console.log(JSON.stringify({ ok: true, type: 'orchestration_scope_validation_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
