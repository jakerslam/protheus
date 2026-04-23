#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../..');

function resetModule(modulePath) {
  delete require.cache[require.resolve(modulePath)];
  return require(modulePath);
}

function main() {
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), 'training-conduit-rust-'));
  const clientRoot = path.join(workspace, 'client');
  const policyPath = path.join(clientRoot, 'config', 'training_conduit_policy.json');
  fs.mkdirSync(path.dirname(policyPath), { recursive: true });
  fs.writeFileSync(policyPath, JSON.stringify({
    defaults: {
      owner_id: 'operator-x',
      retention_days: 30,
      consent_evidence_ref: 'config/training_conduit_policy.json'
    },
    constraints: {
      min_retention_days: 5,
      max_retention_days: 90
    }
  }, null, 2));

  process.env.INFRING_OPS_USE_PREBUILT = '0';
  process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = '120000';

  const mod = resetModule(path.join(ROOT, 'client/lib/training_conduit_schema.ts'));
  const defaults = mod.defaultPolicy();
  assert.equal(typeof defaults.defaults.owner_id, 'string');
  assert.equal(defaults.defaults.owner_id.length > 0, true);
  assert.equal(typeof defaults.constraints.max_retention_days, 'number');

  const policy = mod.loadTrainingConduitPolicy(policyPath);
  assert.equal(policy.defaults.owner_id, 'operator-x');
  assert.equal(policy.defaults.retention_days, 30);

  const normalized = mod.normalizePolicy({
    defaults: {
      owner_id: ' Team Lead ',
      retention_days: 999,
      consent_status: 'Granted'
    },
    constraints: {
      min_retention_days: 7,
      max_retention_days: 60
    }
  });
  assert.equal(normalized.defaults.owner_id, 'team_lead');
  assert.equal(normalized.defaults.retention_days, 999);
  assert.equal(normalized.defaults.consent_status, 'granted');
  assert.equal(normalized.constraints.max_retention_days, 60);

  const metadata = mod.buildTrainingConduitMetadata({
    ts: '2026-03-17T00:00:00.000Z',
    source_system: 'discord',
    source_channel: 'ops',
    datum_id: 'abc-123',
    delete_key: ' custom key '
  }, policy);
  assert.equal(metadata.source.system, 'discord');
  assert.equal(metadata.source.channel, 'ops');
  assert.equal(metadata.delete.key, 'custom_key');
  assert.equal(metadata.consent.status, 'granted');
  assert.equal(metadata.validation.ok, true);

  const invalid = mod.validateTrainingConduitMetadata({
    source: {},
    owner: {},
    license: {},
    consent: {},
    retention: { days: 0 },
    delete: {}
  }, policy);
  assert.equal(invalid.ok, false);
  assert.equal(invalid.policy_version, '1.0');
  assert.ok(invalid.errors.includes('missing_source_system'));
  assert.ok(invalid.errors.includes('missing_delete_key'));

  console.log(JSON.stringify({ ok: true, type: 'training_conduit_schema_rust_bridge_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
