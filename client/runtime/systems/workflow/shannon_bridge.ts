#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::shannon-bridge (authoritative shared workflow bridge).

const { createCompatWorkflowExportBridge } = require('../../lib/compat_target_bridge.ts');
const bridge = createCompatWorkflowExportBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../lib/shannon_bridge.ts',
  loadError: 'shannon_bridge_target_load_failed',
  invalidError: 'shannon_bridge_target_invalid',
  framework: 'shannon',
  bridgePath: 'client/runtime/lib/shannon_bridge.ts',
  bridgeTarget: 'adapters/runtime/shannon_bridge.ts'
});
bridge.exitIfMain(module);

module.exports = bridge.exported;
