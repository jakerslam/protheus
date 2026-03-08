#!/usr/bin/env node
'use strict';

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge');

const bridge = createOpsLaneBridge(__dirname, 'sensory_eyes_intake', 'sensory-eyes-intake');

function usage() {
  process.stdout.write('Usage: eyes_intake.js create|validate|list-directives [options]\n');
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
