#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin protocol bridge over pydantic-ai-bridge)

const bridge = require('../../client/runtime/systems/workflow/pydantic_ai_bridge.ts');
const BRIDGE_PATH = 'adapters/protocol/pydantic_ai_protocol_bridge.ts';
const FRAMEWORK = 'pydantic_ai';

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

function registerToolContext(payload = {}) {
  return bridge.registerToolContext(withBridgeMetadata(payload));
}

function invokeToolContext(payload = {}) {
  return bridge.invokeToolContext(withBridgeMetadata(payload));
}

function bridgeProtocol(payload = {}) {
  return bridge.bridgeProtocol(withBridgeMetadata(payload));
}

function streamModel(payload = {}) {
  return bridge.streamModel(withBridgeMetadata(payload));
}

function runGovernedWorkflow(payload = {}) {
  return bridge.runGovernedWorkflow(withBridgeMetadata(payload));
}

module.exports = {
  BRIDGE_PATH,
  FRAMEWORK,
  withBridgeMetadata,
  status,
  registerToolContext,
  invokeToolContext,
  bridgeProtocol,
  streamModel,
  runGovernedWorkflow,
};
