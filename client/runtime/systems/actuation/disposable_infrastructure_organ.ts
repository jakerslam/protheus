#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-ACTUATION-DISPOSABLE_INFRASTRUCTURE_ORGAN';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'disposable_infrastructure_organ',
  systemId: SYSTEM_ID,
  type: 'disposable_infrastructure_organ',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
