#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer1/security (authoritative)

const { runSecurityPlane, runSecurityPlaneCli } = require('../../lib/security_plane_bridge');

function run(args = []) {
  return runSecurityPlane('remote-emergency-halt', args);
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/remote_emergency_halt.js status');
  console.log('  node systems/security/remote_emergency_halt.js sign-halt --approval-note=<text> [--scope=<scope>] [--ttl-sec=<n>]');
  console.log('  node systems/security/remote_emergency_halt.js sign-purge --pending-id=<id>');
  console.log('  node systems/security/remote_emergency_halt.js receive --command=<json>');
  console.log('  node systems/security/remote_emergency_halt.js receive-b64 --command-b64=<base64>');
}

if (require.main === module) {
  const args = process.argv.slice(2);
  const cmd = String(args[0] || '').trim().toLowerCase();
  if (!cmd || cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    process.exit(0);
  }
  runSecurityPlaneCli('remote-emergency-halt', args);
}

module.exports = {
  run
};
