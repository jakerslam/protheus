#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin connector bridge over workflow_chain-bridge)

const bridge = require('../../client/runtime/systems/workflow/workflow_chain_bridge.ts');
const BRIDGE_PATH = 'adapters/protocol/workflow_chain_connector_bridge.ts';
const FRAMEWORK = 'workflow_chain';

function withBridgeMetadata(payload = {}) {
  return {
    bridge_path: BRIDGE_PATH,
    framework: FRAMEWORK,
    ...payload,
  };
}

function status(payload = {}) {
  return bridge.status(withBridgeMetadata(payload));
}

function importIntegration(payload = {}) {
  return bridge.importIntegration(withBridgeMetadata(payload));
}

function registerMemoryBridge(payload = {}) {
  return bridge.registerMemoryBridge(withBridgeMetadata(payload));
}

function recallMemory(payload = {}) {
  return bridge.recallMemory(withBridgeMetadata(payload));
}

module.exports = {
  BRIDGE_PATH,
  FRAMEWORK,
  withBridgeMetadata,
  status,
  importIntegration,
  registerMemoryBridge,
  recallMemory,
};
