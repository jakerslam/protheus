#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-OBSIDIAN-OBSIDIAN_BRIDGE';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'obsidian_bridge',
  systemId: SYSTEM_ID,
  type: 'obsidian_bridge',
  maxArgLen: 512,
  maxArgs: 64,
  inheritStdio: true
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
