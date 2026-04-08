#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../..');
const BRIDGE_PATH = path.join(ROOT, 'client/runtime/lib/rust_lane_bridge.ts');
const MODULE_PATH = path.join(ROOT, 'client/runtime/systems/memory/memory_efficiency_plane.ts');

function withBridgeStub(stubRun, testFn) {
  const bridgeModule = require(BRIDGE_PATH);
  const originalCreateOpsLaneBridge = bridgeModule.createOpsLaneBridge;
  const originalCreateManifestLaneBridge = bridgeModule.createManifestLaneBridge;

  bridgeModule.createOpsLaneBridge = function createOpsLaneBridgeStub() {
    return {
      lane: 'memory-guard-test-lane',
      run: stubRun
    };
  };
  bridgeModule.createManifestLaneBridge = function createManifestLaneBridgeStub() {
    return {
      lane: 'memory-guard-test-manifest-lane',
      run: stubRun,
      runCli() {}
    };
  };

  delete require.cache[MODULE_PATH];
  try {
    return testFn(require(MODULE_PATH));
  } finally {
    delete require.cache[MODULE_PATH];
    bridgeModule.createOpsLaneBridge = originalCreateOpsLaneBridge;
    bridgeModule.createManifestLaneBridge = originalCreateManifestLaneBridge;
  }
}

function main() {
  let bridgeCalls = 0;

  withBridgeStub(() => {
    bridgeCalls += 1;
    return {
      ok: true,
      status: 0,
      stdout: '',
      stderr: '',
      payload: { ok: true, type: 'stub_bridge' }
    };
  }, (memoryEfficiencyPlane) => {
    // Rust-core bypass attempt must be rejected before the bridge can run.
    const bypassResult = memoryEfficiencyPlane.run([
      'query-index',
      '--session-id=session-security-test',
      '--bypass=1'
    ]);
    assert.strictEqual(bypassResult.status, 2);
    assert.strictEqual(
      bypassResult.payload && bypassResult.payload.reason,
      'index_first_bypass_forbidden'
    );
    assert.strictEqual(bridgeCalls, 0, 'bridge should not run on policy bypass');

    const validResult = memoryEfficiencyPlane.run([
      'query-index',
      '--session-id=session-security-test',
      '--top=5',
      '--max-files=1'
    ]);
    assert.strictEqual(validResult.status, 0);
    assert.strictEqual(bridgeCalls, 1, 'bridge should run when guard passes');
  });

  console.log(
    JSON.stringify({
      ok: true,
      type: 'memory_security_gate_integration_test'
    })
  );
}

if (require.main === module) {
  main();
}

module.exports = { main };
