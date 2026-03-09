#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer1/security (authoritative)

const { runSecurityPlane, runSecurityPlaneCli } = require('../../lib/security_plane_bridge');

function run(args = []) {
  return runSecurityPlane('constitution-guardian', args);
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/constitution_guardian.js init-genesis [--force=1|0]');
  console.log('  node systems/security/constitution_guardian.js propose-change --candidate-file=<path> --proposer-id=<id> --reason=<text>');
  console.log('  node systems/security/constitution_guardian.js approve-change --proposal-id=<id> --approver-id=<id> --approval-note=<text>');
  console.log('  node systems/security/constitution_guardian.js veto-change --proposal-id=<id> --veto-by=<id> --note=<text>');
  console.log('  node systems/security/constitution_guardian.js run-gauntlet --proposal-id=<id> [--critical-failures=<n>] [--evidence=<text>]');
  console.log('  node systems/security/constitution_guardian.js activate-change --proposal-id=<id> --approver-id=<id> --approval-note=<text>');
  console.log('  node systems/security/constitution_guardian.js enforce-inheritance --actor=<id> --target=<id>');
  console.log('  node systems/security/constitution_guardian.js emergency-rollback --note=<text>');
  console.log('  node systems/security/constitution_guardian.js status');
}

if (require.main === module) {
  const args = process.argv.slice(2);
  const cmd = String(args[0] || '').trim().toLowerCase();
  if (!cmd || cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    process.exit(0);
  }
  runSecurityPlaneCli('constitution-guardian', args);
}

module.exports = {
  run
};
