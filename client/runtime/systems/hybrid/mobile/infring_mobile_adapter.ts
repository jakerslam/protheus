#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-HYBRID-MOBILE-INFRING_MOBILE_ADAPTER';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'infring_mobile_adapter',
  systemId: SYSTEM_ID,
  type: 'infring_mobile_adapter',
  maxArgLen: 512,
  maxArgs: 64,
  inheritStdio: true
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
