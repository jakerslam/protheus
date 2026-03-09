#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer1/security (authoritative)

const { runSecurityPlane, runSecurityPlaneCli } = require('../../lib/security_plane_bridge');

function run(args = []) {
  return runSecurityPlane('integrity-reseal', args);
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/integrity_reseal.js check [--policy=<path>] [--staged=1|0]');
  console.log('  node systems/security/integrity_reseal.js apply [--policy=<path>] [--approval-note="..."] [--force=1]');
}

if (require.main === module) {
  const args = process.argv.slice(2);
  const cmd = String(args[0] || '').trim().toLowerCase();
  if (!cmd || cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    process.exit(0);
  }
  runSecurityPlaneCli('integrity-reseal', args);
}

module.exports = {
  run
};
