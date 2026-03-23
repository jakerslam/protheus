#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../..');
const scope = require(path.join(ROOT, 'client/cognition/orchestration/scope.ts'));

function main() {
  const overlapping = scope.detectScopeOverlaps([
    { scope_id: 'v6-sec-a', series: ['V6-SEC'], paths: ['core/layer0/ops/*'] },
    { scope_id: 'v6-sec-b', series: ['V6-SEC'], paths: ['core/layer0/*'] }
  ]);

  assert.strictEqual(overlapping.ok, false);
  assert.strictEqual(overlapping.reason_code, 'scope_overlap_detected');
  assert.strictEqual(overlapping.overlaps.length, 1);
  assert.strictEqual(overlapping.overlaps[0].left_scope_id, 'v6-sec-a');

  const nonOverlapping = scope.detectScopeOverlaps([
    { scope_id: 'nexus', series: ['V7-NEXUS'], paths: ['core/layer0/ops/src/nexus_plane.rs'] },
    { scope_id: 'business', series: ['V7-BUSINESS'], paths: ['core/layer0/ops/src/business_plane.rs'] }
  ]);

  assert.strictEqual(nonOverlapping.ok, true);
  assert.strictEqual(nonOverlapping.overlaps.length, 0);

  console.log(JSON.stringify({ ok: true, type: 'orchestration_scope_overlap_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
