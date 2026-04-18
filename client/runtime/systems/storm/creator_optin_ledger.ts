#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-STORM-CREATOR_OPTIN_LEDGER';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'creator_optin_ledger',
  systemId: SYSTEM_ID,
  type: 'creator_optin_ledger',
  maxArgLen: 512,
  maxArgs: 64,
  inheritStdio: true
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
