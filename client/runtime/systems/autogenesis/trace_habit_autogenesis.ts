#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-AUTOGENESIS-TRACE_HABIT_AUTOGENESIS';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'trace_habit_autogenesis',
  systemId: SYSTEM_ID,
  type: 'trace_habit_autogenesis',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
