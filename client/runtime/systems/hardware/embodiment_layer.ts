#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-HARDWARE-EMBODIMENT_LAYER';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'embodiment_layer',
  systemId: SYSTEM_ID,
  type: 'embodiment_layer',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
