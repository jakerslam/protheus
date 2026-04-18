#!/usr/bin/env node
'use strict';
const { createRuntimeSystemEntrypoint } = require('../../lib/runtime_system_entrypoint.ts');

const SYSTEM_ID = 'SYSTEMS-HARDWARE-COMPRESSION_TRANSFER_PLANE';
const entrypoint = createRuntimeSystemEntrypoint(__dirname, {
  lane: 'compression_transfer_plane',
  systemId: SYSTEM_ID,
  type: 'compression_transfer_plane',
  maxArgs: 64,
  maxArgLen: 512
});

if (require.main === module) {
  entrypoint.exitFromRun(process.argv.slice(2));
}

module.exports = entrypoint;
