#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin MCP interop bridge over mastra-bridge)

const bridge = require('../../client/runtime/systems/workflow/mastra_bridge.ts');
const BRIDGE_PATH = 'adapters/protocol/mastra_mcp_bridge.ts';
const FRAMEWORK = 'mastra';

function withBridgeMetadata(payload = {}) {
  return {
    bridge_path: BRIDGE_PATH,
    framework: FRAMEWORK,
    ...payload,
  };
}

function registerBridge(payload = {}) {
  return bridge.registerMcpBridge(withBridgeMetadata(payload));
}

function invokeBridge(payload = {}) {
  return bridge.invokeMcpBridge(withBridgeMetadata(payload));
}

function runGovernedWorkflow(payload = {}) {
  return bridge.runGovernedWorkflow(withBridgeMetadata(payload));
}

module.exports = {
  registerBridge,
  invokeBridge,
  runGovernedWorkflow,
};
