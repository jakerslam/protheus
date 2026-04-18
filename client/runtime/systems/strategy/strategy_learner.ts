#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: surface/orchestration; this file is a thin CLI bridge.

const { createCompatTargetBridge } = require('../../lib/compat_target_bridge.ts');
const bridge = createCompatTargetBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../../../surface/orchestration/scripts/strategy_learner.ts',
  loadError: 'strategy_learner_target_load_failed',
  unavailableError: 'strategy_learner_target_unavailable',
  missingRunError: 'strategy_learner_target_missing_run',
  maxArgLen: 512,
  maxArgs: 64,
});

if (require.main === module) {
  bridge.runAsMain(process.argv.slice(2));
}

module.exports = {
  ...(bridge.target && typeof bridge.target === 'object' ? bridge.target : {}),
  run: bridge.run
};
