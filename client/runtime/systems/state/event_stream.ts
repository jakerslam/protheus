#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-STATE-EVENT_STREAM';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'event_stream',
  systemId: SYSTEM_ID,
  type: 'event_stream',
  maxArgLen: 512,
  maxArgs: 64,
  inheritStdio: true
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
