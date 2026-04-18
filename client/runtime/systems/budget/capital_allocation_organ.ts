#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-BUDGET-CAPITAL_ALLOCATION_ORGAN';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'capital_allocation_organ',
  systemId: SYSTEM_ID,
  type: 'capital_allocation_organ',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
