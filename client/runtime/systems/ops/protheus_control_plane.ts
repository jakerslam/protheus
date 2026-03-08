#!/usr/bin/env node
'use strict';
export {};

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge');

const bridge = createOpsLaneBridge(__dirname, 'protheus_control_plane', 'protheus-control-plane');

function usage() {
  process.stdout.write('Usage: protheus_control_plane.js protheus start|status|job-submit|incident|release-promote|doctor-bundle [options]\n');
}

if (require.main === module) {
  const args = process.argv.slice(2);
  const first = String(args[0] || '').trim().toLowerCase();
  if (!first || first === '--help' || first === '-h' || first === 'help') {
    usage();
    process.exit(0);
  }
  bridge.runCli(args);
}

module.exports = {
  lane: bridge.lane,
  run: bridge.run
};
