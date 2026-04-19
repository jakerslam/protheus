#!/usr/bin/env tsx

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::agent-ws-bridge (dashboard websocket surface adapter).

const {
  createCompatModuleExportBridge
} = require('../../lib/compat_target_bridge.ts');

const dashboardAgentWsBridge = createCompatModuleExportBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../../../adapters/runtime/agent_ws_bridge.ts',
  loadError: 'dashboard_agent_ws_bridge_load_failed',
  invalidError: 'dashboard_agent_ws_bridge_invalid'
});

dashboardAgentWsBridge.exitIfMain(module);
module.exports = dashboardAgentWsBridge.exported;
