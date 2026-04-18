#!/usr/bin/env node
'use strict';

const { createCompatWorkflowExportBridge } = require('../../lib/compat_target_bridge.ts');
const bridge = createCompatWorkflowExportBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../lib/pydantic_ai_bridge.ts',
  loadError: 'pydantic_ai_bridge_target_load_failed',
  invalidError: 'pydantic_ai_bridge_target_invalid',
  framework: 'pydantic_ai',
  bridgePath: 'client/runtime/lib/pydantic_ai_bridge.ts',
  bridgeTarget: 'adapters/runtime/pydantic_ai_bridge.ts'
});
bridge.exitIfMain(module);

module.exports = bridge.exported;
