#!/usr/bin/env node
'use strict';

const { createCompatWorkflowExportBridge } = require('../../lib/compat_target_bridge.ts');
const bridge = createCompatWorkflowExportBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../lib/mastra_bridge.ts',
  loadError: 'mastra_bridge_target_load_failed',
  invalidError: 'mastra_bridge_target_invalid',
  framework: 'mastra',
  bridgePath: 'client/runtime/lib/mastra_bridge.ts',
  bridgeTarget: 'adapters/runtime/mastra_bridge.ts'
});
bridge.exitIfMain(module);

module.exports = bridge.exported;
