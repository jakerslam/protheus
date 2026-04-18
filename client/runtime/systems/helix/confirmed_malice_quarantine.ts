#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-HELIX-CONFIRMED_MALICE_QUARANTINE';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'confirmed_malice_quarantine',
  systemId: SYSTEM_ID,
  type: 'confirmed_malice_quarantine',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
