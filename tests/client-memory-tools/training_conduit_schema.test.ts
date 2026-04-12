#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');

require(path.resolve(__dirname, '..', '..', 'client', 'runtime', 'lib', 'ts_bootstrap.ts')).installTsRequireHook();

const schema = require(path.resolve(__dirname, '..', '..', 'client', 'lib', 'training_conduit_schema.ts'));
const proxy = require(path.resolve(__dirname, '..', '..', 'client', 'runtime', 'lib', 'legacy_conduit_proxy.ts'));

function main() {
  assert.equal(typeof schema.defaultPolicy, 'function');
  assert.equal(typeof schema.normalizePolicy, 'function');
  assert.equal(typeof schema.loadTrainingConduitPolicy, 'function');
  assert.equal(typeof schema.buildTrainingConduitMetadata, 'function');
  assert.equal(typeof schema.validateTrainingConduitMetadata, 'function');
  assert.match(schema.DEFAULT_POLICY_PATH, /training_conduit_policy\.json$/);

  const defaults = schema.defaultPolicy();
  assert.equal(typeof defaults.defaults.owner_id, 'string');
  assert.equal(defaults.defaults.owner_id.length > 0, true);

  const normalized = schema.normalizePolicy({
    defaults: {
      owner_id: ' Learning Team ',
      retention_days: 14,
      consent_status: 'Granted',
    },
    constraints: {
      min_retention_days: 7,
      max_retention_days: 30,
    }
  });
  assert.equal(normalized.defaults.owner_id, 'learning_team');
  assert.equal(normalized.defaults.consent_status, 'granted');
  assert.equal(normalized.constraints.min_retention_days, 7);

  const invalid = schema.validateTrainingConduitMetadata({
    source: {},
    owner: {},
    license: {},
    consent: {},
    retention: { days: 0 },
    delete: {},
  });
  assert.equal(invalid.ok, false);
  assert.ok(invalid.errors.includes('missing_delete_key'));

  const runDomain = proxy.createDomainProxy(__dirname, 'IMPORTER_TEST', 'execution-yield-recovery');
  const receipt = runDomain(['status']);
  assert.equal(receipt.ok, true);
  assert.equal(receipt.engine, 'conduit');
  assert.equal(receipt.status, 0);
  assert.equal(receipt.payload.type, 'execution_yield_recovery');
  assert.equal(receipt.payload.command, 'status');
  assert.equal(receipt.payload.routed_via, 'core_local');
  assert.equal(receipt.routed_via, 'core_local');

  console.log(JSON.stringify({ ok: true, type: 'training_conduit_schema_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
