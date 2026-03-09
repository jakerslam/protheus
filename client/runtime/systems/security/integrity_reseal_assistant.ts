#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer1/security (authoritative)

const { runSecurityPlane, runSecurityPlaneCli } = require('../../lib/security_plane_bridge');

function cmdRun(args = {}) {
  const argv = ['run'];
  if (args.apply != null) argv.push(`--apply=${String(args.apply)}`);
  if (args.policy) argv.push(`--policy=${String(args.policy)}`);
  if (args.note) argv.push(`--note=${String(args.note)}`);
  if (args.strict != null) argv.push(`--strict=${String(args.strict)}`);
  const out = runSecurityPlane('integrity-reseal-assistant', argv);
  return out && out.payload ? out.payload : { ok: false, error: 'missing_payload' };
}

function cmdStatus(args = {}) {
  const argv = ['status'];
  if (args.policy) argv.push(`--policy=${String(args.policy)}`);
  const out = runSecurityPlane('integrity-reseal-assistant', argv);
  return out && out.payload ? out.payload : { ok: false, error: 'missing_payload' };
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/integrity_reseal_assistant.js run [--apply=1|0] [--policy=<path>] [--note="..."] [--strict=1|0]');
  console.log('  node systems/security/integrity_reseal_assistant.js status [--policy=<path>]');
}

if (require.main === module) {
  const args = process.argv.slice(2);
  const cmd = String(args[0] || '').trim().toLowerCase();
  if (!cmd || cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    process.exit(0);
  }
  runSecurityPlaneCli('integrity-reseal-assistant', args);
}

module.exports = {
  cmdRun,
  cmdStatus
};
