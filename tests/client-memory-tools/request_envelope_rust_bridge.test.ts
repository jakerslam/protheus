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
  process.env.INFRING_OPS_USE_PREBUILT = '0';
  process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = '120000';

  const mod = resetModule(path.join(ROOT, 'client/lib/request_envelope.ts'));
  assert.deepEqual(mod.normalizeFiles(['b.txt', 'a.txt', 'a.txt', 'c\\x.txt']), ['a.txt', 'b.txt', 'c/x.txt']);
  assert.equal(mod.normalizeKeyId('Prod.Key-01!?'), 'prod.key-01');
  assert.equal(mod.secretKeyEnvVarName('prod.key-01'), 'REQUEST_GATE_SECRET_PROD_KEY_01');

  const payload = mod.envelopePayload({
    source: 'LOCAL',
    action: 'APPLY',
    ts: 1000,
    nonce: 'abc123',
    files: ['a.txt', 'b.txt'],
    kid: 'prod.key-01'
  });
  assert.equal(payload.source, 'local');
  assert.equal(payload.action, 'apply');
  assert.equal(payload.kid, 'prod.key-01');

  const signature = mod.signEnvelope(payload, 'secret-123');
  const verified = mod.verifyEnvelope({
    ...payload,
    signature,
    secret: 'secret-123',
    nowSec: 1000
  });
  assert.equal(verified.ok, true);

  const env = mod.stampGuardEnv({ REQUEST_GATE_SECRET: 'secret-123' }, {
    source: 'local',
    action: 'apply',
    files: ['a.txt'],
    ts: 1000,
    nonce: 'seeded'
  });
  assert.equal(typeof env.REQUEST_SIG, 'string');
  const envVerify = mod.verifySignedEnvelopeFromEnv({
    env,
    files: ['a.txt'],
    nowSec: 1000
  });
  assert.equal(envVerify.ok, true);
  assertNoPlaceholderOrPromptLeak({ payload, verified, envVerify }, 'request_envelope_rust_bridge_test');
  assertStableToolingEnvelope(verified, 'request_envelope_rust_bridge_test');
  console.log(JSON.stringify({ ok: true, type: 'request_envelope_rust_bridge_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
