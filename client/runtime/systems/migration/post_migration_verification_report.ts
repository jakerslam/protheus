#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-MIGRATION-POST_MIGRATION_VERIFICATION_REPORT';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'post_migration_verification_report',
  systemId: SYSTEM_ID,
  type: 'post_migration_verification_report',
  maxArgLen: 512,
  maxArgs: 64,
  inheritStdio: true
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
