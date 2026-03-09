#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer1/security (authoritative)

const { runSecurityPlane, runSecurityPlaneCli } = require('../../lib/security_plane_bridge');

function run(args = []) {
  return runSecurityPlane('soul-token-guard', args);
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/soul_token_guard.js issue [--instance-id=<id>] [--approval-note=<text>]');
  console.log('  node systems/security/soul_token_guard.js stamp-build --build-id=<id> [--channel=<name>] [--valid-hours=<n>]');
  console.log('  node systems/security/soul_token_guard.js verify [--strict=1]');
  console.log('  node systems/security/soul_token_guard.js status');
}

if (require.main === module) {
  const args = process.argv.slice(2);
  const cmd = String(args[0] || '').trim().toLowerCase();
  if (!cmd || cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    process.exit(0);
  }
  runSecurityPlaneCli('soul-token-guard', args);
}

module.exports = {
  run
};
