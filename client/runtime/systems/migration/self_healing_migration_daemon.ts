#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-MIGRATION-SELF_HEALING_MIGRATION_DAEMON';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'self_healing_migration_daemon',
  systemId: SYSTEM_ID,
  type: 'self_healing_migration_daemon',
  maxArgLen: 512,
  maxArgs: 64,
  inheritStdio: true
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
