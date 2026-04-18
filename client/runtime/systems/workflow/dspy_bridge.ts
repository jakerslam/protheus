#!/usr/bin/env node
'use strict';

const { createCompatWorkflowExportBridge } = require('../../lib/compat_target_bridge.ts');
const bridge = createCompatWorkflowExportBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../lib/dspy_bridge.ts',
  loadError: 'dspy_bridge_target_load_failed',
  invalidError: 'dspy_bridge_target_invalid',
  framework: 'dspy',
  bridgePath: 'client/runtime/lib/dspy_bridge.ts',
  bridgeTarget: 'adapters/runtime/dspy_bridge.ts'
});
bridge.exitIfMain(module);

module.exports = bridge.exported;
