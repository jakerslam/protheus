#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-STORM-STORM_VALUE_DISTRIBUTION';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'storm_value_distribution',
  systemId: SYSTEM_ID,
  type: 'storm_value_distribution',
  maxArgLen: 512,
  maxArgs: 64,
  inheritStdio: true
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
