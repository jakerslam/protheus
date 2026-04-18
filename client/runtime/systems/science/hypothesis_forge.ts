#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-SCIENCE-HYPOTHESIS_FORGE';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'hypothesis_forge',
  systemId: SYSTEM_ID,
  type: 'hypothesis_forge',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
