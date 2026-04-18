#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-SOUL-SOUL_CONTINUITY_ADAPTER';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'soul_continuity_adapter',
  systemId: SYSTEM_ID,
  type: 'soul_continuity_adapter',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
