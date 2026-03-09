#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer1/security (authoritative)

const { runSecurityPlaneCli } = require('../../lib/security_plane_bridge');

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/startup_attestation.js issue [--ttl-hours=<n>] [--strict=1|0]');
  console.log('  node systems/security/startup_attestation.js verify [--strict=1|0]');
  console.log('  node systems/security/startup_attestation.js status');
}

if (require.main === module) {
  const args = process.argv.slice(2);
  const cmd = String(args[0] || '').trim().toLowerCase();
  if (!cmd || cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    process.exit(0);
  }
  runSecurityPlaneCli('startup-attestation', args);
}

module.exports = {};
