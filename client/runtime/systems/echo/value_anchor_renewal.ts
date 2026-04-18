#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-ECHO-VALUE_ANCHOR_RENEWAL';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'value_anchor_renewal',
  systemId: SYSTEM_ID,
  type: 'value_anchor_renewal',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
