#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::shannon-desktop-shell (authoritative workflow desktop bridge).

const { createCompatWorkflowExportBridge } = require('../../lib/compat_target_bridge.ts');
const bridge = createCompatWorkflowExportBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../../../adapters/runtime/shannon_desktop_shell.ts',
  loadError: 'shannon_desktop_shell_target_load_failed',
  invalidError: 'shannon_desktop_shell_target_invalid',
  framework: 'shannon_desktop_shell',
  bridgePath: 'client/runtime/systems/workflow/shannon_desktop_shell.ts',
  bridgeTarget: 'adapters/runtime/shannon_desktop_shell.ts'
});
bridge.exitIfMain(module);

module.exports = bridge.exported;
