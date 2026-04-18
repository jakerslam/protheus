#!/usr/bin/env node
'use strict';

const { createCompatWorkflowExportBridge } = require('../../lib/compat_target_bridge.ts');
const bridge = createCompatWorkflowExportBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../lib/semantic_kernel_bridge.ts',
  loadError: 'semantic_kernel_bridge_target_load_failed',
  invalidError: 'semantic_kernel_bridge_target_invalid',
  framework: 'semantic_kernel',
  bridgePath: 'client/runtime/lib/semantic_kernel_bridge.ts',
  bridgeTarget: 'adapters/runtime/semantic_kernel_bridge.ts'
});
bridge.exitIfMain(module);

module.exports = bridge.exported;
