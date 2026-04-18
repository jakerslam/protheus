#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: surface/orchestration; this file is a thin CLI bridge.

const { createCompatTargetBridge } = require('../../lib/compat_target_bridge.ts');
const bridge = createCompatTargetBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../../../surface/orchestration/scripts/experiment_scheduler.ts',
  loadError: 'experiment_scheduler_target_load_failed',
  unavailableError: 'experiment_scheduler_target_unavailable',
  missingRunError: 'experiment_scheduler_target_missing_run',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  bridge.runAsMain(process.argv.slice(2));
}

module.exports = {
  ...(bridge.target && typeof bridge.target === 'object' ? bridge.target : {}),
  run: bridge.run,
  normalizeReceiptHash: bridge.normalizeReceiptHash
};
