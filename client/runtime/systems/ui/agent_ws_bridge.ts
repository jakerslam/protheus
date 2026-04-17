#!/usr/bin/env tsx

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::agent-ws-bridge (dashboard websocket surface adapter).

const path = require('node:path');

const TARGET = path.resolve(__dirname, '../../../../adapters/runtime/agent_ws_bridge.ts');

function loadAgentWsBridge() {
  try {
    return require(TARGET);
  } catch (error) {
    const message = String(error && error.message ? error.message : error || 'unknown');
    const err = new Error('agent_ws_bridge_load_failed: ' + message);
    err.code = 'agent_ws_bridge_load_failed';
    throw err;
  }
}

module.exports = loadAgentWsBridge();
