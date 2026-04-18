#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-FRACTAL-MINI_CORE_INSTANCER';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'mini_core_instancer',
  systemId: SYSTEM_ID,
  type: 'mini_core_instancer',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
