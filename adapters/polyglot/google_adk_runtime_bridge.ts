#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/polyglot (thin interop bridge over google-adk-bridge)

const bridge = require('../../client/runtime/systems/workflow/google_adk_bridge.ts');
const BRIDGE_PATH = 'adapters/polyglot/google_adk_runtime_bridge.ts';
const FRAMEWORK = 'google_adk';

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
