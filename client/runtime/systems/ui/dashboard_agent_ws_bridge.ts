#!/usr/bin/env tsx

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::agent-ws-bridge (dashboard websocket surface adapter).

const {
  createCompatModuleExportBridge
} = require('../../lib/compat_target_bridge.ts');

const bridge = createCompatModuleExportBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../../../adapters/runtime/agent_ws_bridge.ts',
  loadError: 'agent_ws_bridge_load_failed',
  invalidError: 'agent_ws_bridge_invalid'
});

bridge.exitIfMain(module);
module.exports = bridge.exported;
