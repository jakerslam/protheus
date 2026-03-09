#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer1/security (authoritative)

const { runSecurityPlane, runSecurityPlaneCli } = require('../../lib/security_plane_bridge');

function run(args = []) {
  return runSecurityPlane('guard', args);
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/guard.js --files=<path1,path2,...> [--strict=1|0]');
  console.log('  node systems/security/guard.js status');
}

if (require.main === module) {
  const args = process.argv.slice(2);
  const cmd = String(args[0] || '').trim().toLowerCase();
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    process.exit(0);
  }
  runSecurityPlaneCli('guard', args);
}

module.exports = {
  run
};
