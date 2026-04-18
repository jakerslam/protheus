#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-WEAVER-DRIFT_AWARE_REVENUE_OPTIMIZER';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'drift_aware_revenue_optimizer',
  systemId: SYSTEM_ID,
  type: 'drift_aware_revenue_optimizer',
  maxArgLen: 512,
  maxArgs: 64,
  inheritStdio: true
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
