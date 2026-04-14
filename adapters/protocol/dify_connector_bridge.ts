#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin connector bridge over dify-bridge)

const bridge = require('../../client/runtime/systems/workflow/dify_bridge.ts');
const BRIDGE_PATH = 'adapters/protocol/dify_connector_bridge.ts';
const FRAMEWORK = 'dify';

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

function syncKnowledgeBase(payload = {}) {
  return bridge.syncKnowledgeBase(withBridgeMetadata(payload));
}

function registerAgentApp(payload = {}) {
  return bridge.registerAgentApp(withBridgeMetadata(payload));
}

function routeProvider(payload = {}) {
  return bridge.routeProvider(withBridgeMetadata(payload));
}

module.exports = {
  BRIDGE_PATH,
  FRAMEWORK,
  withBridgeMetadata,
  status,
  syncKnowledgeBase,
  registerAgentApp,
  routeProvider,
};
