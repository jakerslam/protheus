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
  console.log('  protheusctl rsi bootstrap --owner=jay');
  console.log('  protheusctl rsi step --owner=jay --target-path=systems/strategy/strategy_learner.ts');
  console.log('  protheusctl contract-lane status --owner=jay');
  console.log('  protheusctl approve --rsi --owner=jay --approver=<you>');
}

function runScript(script: string, args: string[] = []) {
  const r = spawnSync('node', [script, ...args], { encoding: 'utf8' });
  if (r.stdout) process.stdout.write(r.stdout);
  if (r.stderr) process.stderr.write(r.stderr);
  process.exit(Number.isFinite(r.status) ? r.status : 1);
}

function main() {
  const argv = process.argv.slice(2);
  const cmd = String(argv[0] || 'status');
  const rest = argv.slice(1);
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    return;
  }

  if (cmd === 'skills' && String(rest[0] || '') === 'discover') {
    const discoverScript = path.join(__dirname, 'protheusctl_skills_discover.js');
    runScript(discoverScript, rest.slice(1));
    return;
  }

  if (cmd === 'rsi') {
    const rsiScript = path.join(__dirname, '..', '..', 'adaptive', 'rsi', 'rsi_bootstrap.js');
    const subcmd = String(rest[0] || 'status');
    runScript(rsiScript, [subcmd, ...rest.slice(1)]);
    return;
  }

  if (cmd === 'contract-lane' && String(rest[0] || '') === 'status') {
    const rsiScript = path.join(__dirname, '..', '..', 'adaptive', 'rsi', 'rsi_bootstrap.js');
    runScript(rsiScript, ['contract-lane-status', ...rest.slice(1)]);
    return;
  }

  if (cmd === 'approve' && rest.includes('--rsi')) {
    const rsiScript = path.join(__dirname, '..', '..', 'adaptive', 'rsi', 'rsi_bootstrap.js');
    runScript(rsiScript, ['approve', ...rest.filter((arg) => arg !== '--rsi')]);
    return;
  }

  const script = path.join(__dirname, 'protheus_control_plane.js');
  runScript(script, [cmd, ...rest]);
}

main();
