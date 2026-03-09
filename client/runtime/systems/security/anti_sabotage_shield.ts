#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer1/security (authoritative)

const { runSecurityPlane, runSecurityPlaneCli } = require('../../lib/security_plane_bridge');

function run(args = []) {
  return runSecurityPlane('anti-sabotage-shield', args);
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/anti_sabotage_shield.js snapshot [--label=<id>]');
  console.log('  node systems/security/anti_sabotage_shield.js verify [--snapshot=latest|<id>] [--strict=1|0] [--auto-reset=1|0]');
  console.log('  node systems/security/anti_sabotage_shield.js watch [--snapshot=latest|<id>] [--strict=1|0] [--auto-reset=1|0] [--interval-ms=<n>] [--iterations=<n>]');
  console.log('  node systems/security/anti_sabotage_shield.js status');
}

if (require.main === module) {
  const args = process.argv.slice(2);
  const cmd = String(args[0] || '').trim().toLowerCase();
  if (!cmd || cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    process.exit(0);
  }
  runSecurityPlaneCli('anti-sabotage-shield', args);
}

module.exports = {
  run
};
