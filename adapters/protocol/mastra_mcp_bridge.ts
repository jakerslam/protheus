#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin MCP interop bridge over mastra-bridge)

const bridge = require('../../client/runtime/systems/workflow/mastra_bridge.ts');

function registerBridge(payload = {}) {
  return bridge.registerMcpBridge({
    bridge_path: 'adapters/protocol/mastra_mcp_bridge.ts',
    ...payload,
  });
}

function invokeBridge(payload = {}) {
  return bridge.invokeMcpBridge(payload);
}

module.exports = {
  registerBridge,
  invokeBridge,
};
