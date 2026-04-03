#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin connector bridge over workflow_chain-bridge)

const bridge = require('../../client/runtime/systems/workflow/workflow_chain_bridge.ts');

function importIntegration(payload = {}) {
  return bridge.importIntegration({
    bridge_path: 'adapters/protocol/workflow_chain_connector_bridge.ts',
    ...payload,
  });
}

function registerMemoryBridge(payload = {}) {
  return bridge.registerMemoryBridge({
    bridge_path: 'adapters/protocol/workflow_chain_connector_bridge.ts',
    ...payload,
  });
}

function recallMemory(payload = {}) {
  return bridge.recallMemory(payload);
}

module.exports = {
  importIntegration,
  registerMemoryBridge,
  recallMemory,
};
