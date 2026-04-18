#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-PERSONAS-SHADOW_CLI';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'shadow_cli',
  systemId: SYSTEM_ID,
  type: 'shadow_cli',
  maxArgLen: 512,
  maxArgs: 64,
  inheritStdio: true
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
