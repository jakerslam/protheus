#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.

const { createCompatTargetBridge } = require('../../lib/compat_target_bridge.ts');
const bridge = createCompatTargetBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../../../surface/orchestration/scripts/economic_entity_manager.ts',
  loadError: 'economic_entity_manager_target_load_failed',
  unavailableError: 'economic_entity_manager_target_unavailable',
  missingRunError: 'economic_entity_manager_target_missing_run',
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
