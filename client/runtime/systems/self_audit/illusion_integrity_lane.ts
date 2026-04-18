#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-SELF_AUDIT-ILLUSION_INTEGRITY_LANE';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'illusion_integrity_lane',
  systemId: SYSTEM_ID,
  type: 'illusion_integrity_lane',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
