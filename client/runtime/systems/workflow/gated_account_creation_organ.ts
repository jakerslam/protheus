#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: surface/orchestration; this file is a thin CLI bridge.

const { createCompatTargetBridge } = require('../../lib/compat_target_bridge.ts');
const MAX_ARGS = 64;
const MAX_ARG_LEN = 512;
const bridge = createCompatTargetBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../../../surface/orchestration/scripts/gated_account_creation_organ.ts',
  loadError: 'gated_account_creation_organ_target_load_failed',
  unavailableError: 'gated_account_creation_organ_target_unavailable',
  missingRunError: 'gated_account_creation_organ_target_missing_run',
  maxArgs: MAX_ARGS,
  maxArgLen: MAX_ARG_LEN
});

if (require.main === module) {
  bridge.runAsMain(process.argv.slice(2));
}

module.exports = {
  ...(bridge.target && typeof bridge.target === 'object' ? bridge.target : {}),
  run: bridge.run,
  normalizeReceiptHash: bridge.normalizeReceiptHash
};
