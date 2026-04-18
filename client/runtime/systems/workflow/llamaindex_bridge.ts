#!/usr/bin/env node
'use strict';

const { createCompatWorkflowExportBridge } = require('../../lib/compat_target_bridge.ts');
const bridge = createCompatWorkflowExportBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../lib/llamaindex_bridge.ts',
  loadError: 'llamaindex_bridge_target_load_failed',
  invalidError: 'llamaindex_bridge_target_invalid',
  framework: 'llamaindex',
  bridgePath: 'client/runtime/lib/llamaindex_bridge.ts',
  bridgeTarget: 'adapters/runtime/llamaindex_bridge.ts'
});
bridge.exitIfMain(module);

module.exports = bridge.exported;
