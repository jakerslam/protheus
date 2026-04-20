#!/usr/bin/env tsx

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::dashboard-asset-router (authoritative dashboard host asset composition).

const {
  createCompatModuleExportBridge
} = require('../../lib/compat_target_bridge.ts');

const bridge = createCompatModuleExportBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../../../adapters/runtime/dashboard_asset_router.ts',
  loadError: 'dashboard_asset_router_load_failed',
  invalidError: 'dashboard_asset_router_invalid'
});

bridge.exitIfMain(module);
module.exports = bridge.exported;
