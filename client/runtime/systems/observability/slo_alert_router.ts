#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-OBSERVABILITY-SLO_ALERT_ROUTER';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'slo_alert_router',
  systemId: SYSTEM_ID,
  type: 'slo_alert_router',
  maxArgLen: 512,
  maxArgs: 64,
  inheritStdio: true
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
