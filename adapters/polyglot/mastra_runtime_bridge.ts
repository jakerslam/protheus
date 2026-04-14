#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/polyglot (thin runtime bridge over mastra-bridge)

const bridge = require('../../client/runtime/systems/workflow/mastra_bridge.ts');
const BRIDGE_PATH = 'adapters/polyglot/mastra_runtime_bridge.ts';
const FRAMEWORK = 'mastra';

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

function registerBridge(payload = {}) {
  return bridge.registerRuntimeBridge(withBridgeMetadata(payload));
}

function routeModel(payload = {}) {
  return bridge.routeModel(withBridgeMetadata(payload));
}

module.exports = {
  BRIDGE_PATH,
  FRAMEWORK,
  withBridgeMetadata,
  status,
  registerBridge,
  routeModel,
};
