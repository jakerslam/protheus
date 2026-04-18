#!/usr/bin/env node
'use strict';

const { createCompatWorkflowExportBridge } = require('../../lib/compat_target_bridge.ts');
const bridge = createCompatWorkflowExportBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../lib/metagpt_bridge.ts',
  loadError: 'metagpt_bridge_target_load_failed',
  invalidError: 'metagpt_bridge_target_invalid',
  framework: 'metagpt',
  bridgePath: 'client/runtime/lib/metagpt_bridge.ts',
  bridgeTarget: 'adapters/runtime/metagpt_bridge.ts'
});
bridge.exitIfMain(module);

module.exports = bridge.exported;
