#!/usr/bin/env node
'use strict';
export {};

const { createCompatTargetBridge } = require('../lib/compat_target_bridge.ts');

const bridge = createCompatTargetBridge({
  scriptDir: __dirname,
  targetRelativePath: '../systems/ops/open_platform_release_pack.ts',
  loadError: 'export_cli_target_load_failed',
  unavailableError: 'export_cli_target_unavailable',
  missingRunError: 'export_cli_target_missing_run',
  maxArgLen: 512,
  maxArgs: 64,
});

if (require.main === module) {
  bridge.runAsMain(process.argv.slice(2));
}

module.exports = {
  ...(bridge.target && typeof bridge.target === 'object' ? bridge.target : {}),
  run: bridge.run,
};
