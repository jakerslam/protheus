#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin connector bridge over camel-bridge)

const bridge = require('../../client/runtime/systems/workflow/camel_bridge.ts');
const BRIDGE_PATH = 'adapters/protocol/camel_connector_bridge.ts';
const FRAMEWORK = 'camel';

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

function importDataset(payload = {}) {
  return bridge.importDataset(withBridgeMetadata(payload));
}

function registerToolGateway(payload = {}) {
  return bridge.registerToolGateway(withBridgeMetadata(payload));
}

function invokeToolGateway(payload = {}) {
  return bridge.invokeToolGateway(withBridgeMetadata(payload));
}

module.exports = {
  BRIDGE_PATH,
  FRAMEWORK,
  withBridgeMetadata,
  status,
  importDataset,
  registerToolGateway,
  invokeToolGateway,
};
