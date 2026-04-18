#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-SYMBIOSIS-PRE_NEURALINK_INTERFACE';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'pre_neuralink_interface',
  systemId: SYSTEM_ID,
  type: 'pre_neuralink_interface',
  maxArgLen: 512,
  maxArgs: 64,
  inheritStdio: true
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
