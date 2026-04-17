#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');\nconst { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

const ROOT = path.resolve(__dirname, '../..');

function resetModule(modulePath) {
  delete require.cache[require.resolve(modulePath)];
  return require(modulePath);
}

function main() {
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), 'capability-lease-rust-'));
  const statePath = path.join(workspace, 'capability_leases.json');
  const auditPath = path.join(workspace, 'capability_leases.jsonl');

  process.env.CAPABILITY_LEASE_KEY = 'test-capability-lease-key';
  process.env.CAPABILITY_LEASE_STATE_PATH = statePath;
  process.env.CAPABILITY_LEASE_AUDIT_PATH = auditPath;
  process.env.PROTHEUS_OPS_USE_PREBUILT = '0';
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = '120000';

  const mod = resetModule(path.join(ROOT, 'client/lib/capability_lease.ts'));
  const issued = mod.issueLease({
    scope: 'deploy',
    target: 'prod',
    issued_by: 'test',
    reason: 'bridge-proof',
    ttl_sec: 120
  });
  assert.equal(issued.ok, true);
  assert.ok(issued.token);

  const verified = mod.verifyLease(issued.token, { scope: 'deploy', target: 'prod' });
  assert.equal(verified.ok, true);
  assert.equal(verified.scope, 'deploy');

  const consumed = mod.verifyLease(issued.token, {
    scope: 'deploy',
    target: 'prod',
    consume: true,
    consume_reason: 'used'
  });
  assert.equal(consumed.ok, true);
  assert.equal(consumed.consumed, true);

  const second = mod.verifyLease(issued.token, { scope: 'deploy' });
  assert.equal(second.ok, false);
  assert.equal(second.error, 'lease_already_consumed');

  const state = mod.loadLeaseState();
  assert.equal(Object.keys(state.issued).length, 1);
  assert.equal(Object.keys(state.consumed).length, 1);
  assert.equal(fs.existsSync(mod.LEASE_AUDIT_PATH), true);

  assertNoPlaceholderOrPromptLeak({ issued, verified, consumed, second, state }, 'capability_lease_rust_bridge_test');\n  assertStableToolingEnvelope(issued, 'capability_lease_rust_bridge_test');\n  console.log(JSON.stringify({ ok: true, type: 'capability_lease_rust_bridge_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
