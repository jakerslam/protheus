#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-SOUL-SOUL_PRINT_MANAGER';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'soul_print_manager',
  systemId: SYSTEM_ID,
  type: 'soul_print_manager',
  maxArgLen: 512,
  maxArgs: 64,
  inheritStdio: true
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
