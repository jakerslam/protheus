#!/usr/bin/env node
'use strict';

const { createCompatWorkflowExportBridge } = require('../../lib/compat_target_bridge.ts');
const bridge = createCompatWorkflowExportBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../lib/google_adk_bridge.ts',
  loadError: 'google_adk_bridge_target_load_failed',
  invalidError: 'google_adk_bridge_target_invalid',
  framework: 'google_adk',
  bridgePath: 'client/runtime/lib/google_adk_bridge.ts',
  bridgeTarget: 'adapters/runtime/google_adk_bridge.ts'
});
bridge.exitIfMain(module);

module.exports = bridge.exported;
