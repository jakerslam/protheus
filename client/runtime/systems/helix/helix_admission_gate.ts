#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-HELIX-HELIX_ADMISSION_GATE';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'helix_admission_gate',
  systemId: SYSTEM_ID,
  type: 'helix_admission_gate',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
