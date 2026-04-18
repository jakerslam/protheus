#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-022 open-platform compatibility surface.
 * Delegates to canonical systems/economy/public_donation_api lane.
 */

const { createCompatTargetBridge } = require('../../lib/compat_target_bridge.ts');
const bridge = createCompatTargetBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../systems/economy/public_donation_api.ts',
  loadError: 'donate_gpu_target_load_failed',
  unavailableError: 'donate_gpu_target_unavailable',
  missingRunError: 'donate_gpu_target_missing_run',
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
