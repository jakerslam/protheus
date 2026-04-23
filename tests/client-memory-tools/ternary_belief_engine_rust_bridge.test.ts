#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const path = require('node:path');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

const ROOT = path.resolve(__dirname, '../..');

function resetModule(modulePath) {
  delete require.cache[require.resolve(modulePath)];
  return require(modulePath);
}

function main() {
  process.env.INFRING_OPS_USE_PREBUILT = '0';
  const mod = resetModule(path.join(ROOT, 'client/runtime/lib/ternary_belief_engine.ts'));

  const evaluated = mod.evaluateTernaryBelief([
    { source: 'policy', trit: 'ok', weight: 1.1, confidence: 0.9 },
    { source: 'health', trit: 1, weight: 1.3, confidence: 0.95 },
    { source: 'risk', trit: 'unknown', weight: 0.7, confidence: 0.8 }
  ], {
    label: 'system_health',
    source_trust: { policy: 1.2, health: 1.1 },
    force_neutral_on_insufficient_evidence: true
  });
  assert.strictEqual(evaluated.schema_id, 'ternary_belief');
  assert.strictEqual(evaluated.trit_label, 'ok');
  assert.strictEqual(evaluated.evidence_count, 3);

  const merged = mod.mergeTernaryBeliefs(
    { trit: 1, score: 0.8, confidence: 0.9 },
    { trit: 1, score: 0.6, confidence: 0.7 },
    { mode: 'cautious', parent_weight: 1, child_weight: 1 }
  );
  assert.strictEqual(merged.trit_label, 'ok');

  const serialized = mod.serializeBeliefResult(evaluated);
  assert.strictEqual(serialized.schema_id, 'ternary_belief_serialized');
  assert.strictEqual(serialized.vector.digits.length, 3);

  assertNoPlaceholderOrPromptLeak({ evaluated, merged, serialized }, 'ternary_belief_engine_rust_bridge_test');
  assertStableToolingEnvelope(evaluated, 'ternary_belief_engine_rust_bridge_test');
  console.log(JSON.stringify({ ok: true, type: 'ternary_belief_engine_rust_bridge_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
