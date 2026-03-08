#!/usr/bin/env node
'use strict';
export {};

const path = require('path');
const { runOpsDomainCommand } = require('../../../lib/spine_conduit_bridge');

const ROOT = path.resolve(__dirname, '..', '..');

function usage() {
  console.log('Usage:');
  console.log('  node systems/adaptive/adaptive_runtime.js tick [--reflex-updates=N] [--strategy-updates=N] [--habit-updates=N] [--source=<id>]');
  console.log('  node systems/adaptive/adaptive_runtime.js status');
}

async function run(args = [], opts: Record<string, any> = {}) {
  const routed = Array.isArray(args) && args.length > 0 ? args : ['status'];
  return runOpsDomainCommand('adaptive-runtime', routed, {
    cwdHint: opts.cwdHint || ROOT,
    runContext: opts.runContext || 'adaptive_surface'
  });
}

async function main() {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    usage();
    process.exit(0);
  }
  const out = await run(args);
  if (out.payload) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  } else if (out.stdout) {
    process.stdout.write(String(out.stdout));
  }
  if (out.stderr) {
    process.stderr.write(String(out.stderr));
    if (!String(out.stderr).endsWith('\n')) process.stderr.write('\n');
  }
  process.exit(Number.isFinite(out.status) ? Number(out.status) : 1);
}

if (require.main === module) {
  main().catch((err: any) => {
    process.stderr.write(`${String(err && err.message ? err.message : err)}\n`);
    process.exit(1);
  });
}

module.exports = {
  run
};
