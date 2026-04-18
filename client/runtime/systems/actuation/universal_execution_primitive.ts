#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-ACTUATION-UNIVERSAL_EXECUTION_PRIMITIVE';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'universal_execution_primitive',
  systemId: SYSTEM_ID,
  type: 'universal_execution_primitive',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
