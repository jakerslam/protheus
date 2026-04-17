#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

const ROOT = path.resolve(__dirname, '../..');

function resetModule(modulePath) {
  delete require.cache[require.resolve(modulePath)];
  return require(modulePath);
}

function main() {
  process.env.PROTHEUS_OPS_USE_PREBUILT = '0';
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = '120000';

  const mod = resetModule(path.join(ROOT, 'client/lib/trit.ts'));
  assert.equal(mod.normalizeTrit('ok'), mod.TRIT_OK);
  assert.equal(mod.tritLabel(-1), 'pain');
  assert.equal(mod.tritFromLabel('ready'), mod.TRIT_OK);
  assert.equal(mod.invertTrit('pain'), mod.TRIT_OK);
  assert.equal(mod.majorityTrit(['ok', 'pain', 'ok']), mod.TRIT_OK);
  assert.equal(mod.consensusTrit(['ok', 'ok']), mod.TRIT_OK);
  assert.equal(mod.consensusTrit(['ok', 'pain']), mod.TRIT_UNKNOWN);
  assert.equal(mod.propagateTrit('ok', 'unknown', { mode: 'strict' }), mod.TRIT_UNKNOWN);
  assert.equal(mod.serializeTrit(mod.TRIT_PAIN), '-1');
  assert.equal(mod.parseSerializedTrit('+'), mod.TRIT_OK);
  const vector = mod.serializeTritVector(['pain', 'unknown', 'ok']);
  assert.equal(vector.digits, '-0+');
  assert.deepEqual(mod.parseTritVector(vector), [mod.TRIT_PAIN, mod.TRIT_UNKNOWN, mod.TRIT_OK]);
  assertNoPlaceholderOrPromptLeak({ vector }, 'trit_rust_bridge_test');
  assertStableToolingEnvelope({ status: 'ok', vector }, 'trit_rust_bridge_test');
  console.log(JSON.stringify({ ok: true, type: 'trit_rust_bridge_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
