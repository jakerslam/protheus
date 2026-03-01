#!/usr/bin/env node
'use strict';
export {};

/**
 * protheusctl
 * Typed control client façade over protheus_control_plane.
 */

const path = require('path');
const { spawnSync } = require('child_process');

function usage() {
  console.log('Usage: protheusctl <command> [flags]');
  console.log('Examples:');
  console.log('  protheus status');
  console.log('  protheus health');
  console.log('  protheusctl job-submit --kind=reconcile');
}

function main() {
  const argv = process.argv.slice(2);
  const cmd = String(argv[0] || 'status');
  const rest = argv.slice(1);
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    return;
  }
  const script = path.join(__dirname, 'protheus_control_plane.js');
  const r = spawnSync('node', [script, cmd, ...rest], { encoding: 'utf8' });
  if (r.stdout) process.stdout.write(r.stdout);
  if (r.stderr) process.stderr.write(r.stderr);
  process.exit(Number.isFinite(r.status) ? r.status : 1);
}

main();
