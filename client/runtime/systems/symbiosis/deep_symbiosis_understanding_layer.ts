#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-SYMBIOSIS-DEEP_SYMBIOSIS_UNDERSTANDING_LAYER';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'deep_symbiosis_understanding_layer',
  systemId: SYSTEM_ID,
  type: 'deep_symbiosis_understanding_layer',
  maxArgLen: 512,
  maxArgs: 64,
  inheritStdio: true
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
