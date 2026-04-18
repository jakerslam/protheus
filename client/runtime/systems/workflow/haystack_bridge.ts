#!/usr/bin/env node
'use strict';

const { createCompatWorkflowExportBridge } = require('../../lib/compat_target_bridge.ts');
const bridge = createCompatWorkflowExportBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../lib/haystack_bridge.ts',
  loadError: 'haystack_bridge_target_load_failed',
  invalidError: 'haystack_bridge_target_invalid',
  framework: 'haystack',
  bridgePath: 'client/runtime/lib/haystack_bridge.ts',
  bridgeTarget: 'adapters/runtime/haystack_bridge.ts'
});
bridge.exitIfMain(module);

module.exports = bridge.exported;
