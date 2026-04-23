'use strict';

// Thin runtime wrapper: authoritative bridge implementation lives in adapters/runtime/ops_lane_bridge.ts.

const bridge = require('../../../adapters/runtime/ops_lane_bridge.ts');

const BRIDGE_PATH = 'adapters/runtime/ops_lane_bridge.ts';

function mirrorLegacyEnvFlag(targetName, legacyName) {
  if (!process.env[targetName] && process.env[legacyName]) {
    process.env[targetName] = String(process.env[legacyName]);
  }
}

function normalizeLegacyBridgeEnv() {
  mirrorLegacyEnvFlag('INFRING_OPS_USE_PREBUILT', 'INFRING_OPS_USE_PREBUILT');
  mirrorLegacyEnvFlag('INFRING_OPS_PREFER_CARGO', 'INFRING_OPS_PREFER_CARGO');
  mirrorLegacyEnvFlag(
    'INFRING_OPS_ALLOW_PROCESS_FALLBACK',
    'INFRING_OPS_ALLOW_PROCESS_FALLBACK'
  );
  mirrorLegacyEnvFlag('INFRING_OPS_LOCAL_TIMEOUT_MS', 'INFRING_OPS_LOCAL_TIMEOUT_MS');
}

function createOpsLaneBridge(scriptDir, lane, domain, opts = {}) {
  normalizeLegacyBridgeEnv();
  return bridge.createOpsLaneBridge(scriptDir, lane, domain, opts);
}

function createManifestLaneBridge(scriptDir, lane, options) {
  normalizeLegacyBridgeEnv();
  return bridge.createManifestLaneBridge(scriptDir, lane, options);
}

module.exports = {
  BRIDGE_PATH,
  normalizeLegacyBridgeEnv,
  createOpsLaneBridge,
  createManifestLaneBridge
};
