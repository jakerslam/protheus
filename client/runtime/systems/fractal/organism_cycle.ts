#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-FRACTAL-ORGANISM_CYCLE';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'organism_cycle',
  systemId: SYSTEM_ID,
  type: 'organism_cycle',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
