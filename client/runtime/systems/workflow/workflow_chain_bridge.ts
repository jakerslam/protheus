#!/usr/bin/env node
'use strict';

const { createCompatWorkflowExportBridge } = require('../../lib/compat_target_bridge.ts');
const bridge = createCompatWorkflowExportBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../lib/workflow_chain_bridge.ts',
  loadError: 'workflow_chain_bridge_target_load_failed',
  invalidError: 'workflow_chain_bridge_target_invalid',
  framework: 'workflow_chain',
  bridgePath: 'client/runtime/lib/workflow_chain_bridge.ts',
  bridgeTarget: 'adapters/runtime/workflow_chain_bridge.ts'
});
bridge.exitIfMain(module);

module.exports = bridge.exported;
