#!/usr/bin/env tsx

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::dashboard-asset-router (authoritative dashboard host asset composition).

const {
  createCompatModuleExportBridge
} = require('../../lib/compat_target_bridge.ts');

const dashboardStaticAssetRouterBridge = createCompatModuleExportBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../../../adapters/runtime/dashboard_asset_router.ts',
  loadError: 'dashboard_static_asset_router_load_failed',
  invalidError: 'dashboard_static_asset_router_invalid'
});

dashboardStaticAssetRouterBridge.exitIfMain(module);
module.exports = dashboardStaticAssetRouterBridge.exported;
