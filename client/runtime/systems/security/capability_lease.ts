#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer1/security (authoritative)

const { runSecurityPlaneCli } = require('../../lib/security_plane_bridge');

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/capability_lease.js issue --scope=<scope> [--target=<target>] [--ttl-sec=<n>] [--issued-by=<id>] [--reason=<text>]');
  console.log('  node systems/security/capability_lease.js verify --token=<token> [--scope=<scope>] [--target=<target>]');
  console.log('  node systems/security/capability_lease.js consume --token=<token> [--scope=<scope>] [--target=<target>] [--reason=<text>]');
}

if (require.main === module) {
  const args = process.argv.slice(2);
  const cmd = String(args[0] || '').trim().toLowerCase();
  if (!cmd || cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    process.exit(0);
  }
  runSecurityPlaneCli('capability-lease', args);
}

module.exports = {};
