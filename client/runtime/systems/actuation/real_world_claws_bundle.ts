#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-ACTUATION-REAL_WORLD_CLAWS_BUNDLE';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'real_world_claws_bundle',
  systemId: SYSTEM_ID,
  type: 'real_world_claws_bundle',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
