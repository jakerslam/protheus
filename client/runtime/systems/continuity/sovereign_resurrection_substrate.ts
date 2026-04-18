#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-CONTINUITY-SOVEREIGN_RESURRECTION_SUBSTRATE';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'sovereign_resurrection_substrate',
  systemId: SYSTEM_ID,
  type: 'sovereign_resurrection_substrate',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
