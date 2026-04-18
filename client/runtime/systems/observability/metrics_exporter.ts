#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-OBSERVABILITY-METRICS_EXPORTER';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'metrics_exporter',
  systemId: SYSTEM_ID,
  type: 'metrics_exporter',
  maxArgLen: 512,
  maxArgs: 64,
  inheritStdio: true
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
