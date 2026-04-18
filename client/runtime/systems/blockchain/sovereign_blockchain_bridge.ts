#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-BLOCKCHAIN-SOVEREIGN_BLOCKCHAIN_BRIDGE';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'sovereign_blockchain_bridge',
  systemId: SYSTEM_ID,
  type: 'sovereign_blockchain_bridge',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
