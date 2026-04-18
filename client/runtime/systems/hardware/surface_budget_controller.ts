#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-HARDWARE-SURFACE_BUDGET_CONTROLLER';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'surface_budget_controller',
  systemId: SYSTEM_ID,
  type: 'surface_budget_controller',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
