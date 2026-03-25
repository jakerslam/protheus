#!/usr/bin/env node
'use strict';

// Thin bridge to core authority: rust50-migration-program domain.

const { spawnSync } = require('node:child_process');

function normalizeSubcommand(raw) {
  const sub = String(raw || 'status').trim().toLowerCase();
  if (!sub) return 'status';
  return sub;
}

function runProtheusOpsBridge(args) {
  const proc = spawnSync(
    'node',
    [
      'client/runtime/lib/ts_entrypoint.ts',
      'client/runtime/systems/ops/run_protheus_ops.ts',
      ...args,
    ],
    {
      cwd: process.cwd(),
      env: process.env,
      stdio: 'inherit',
    }
  );
  if (proc && proc.error) return 1;
  return Number.isFinite(proc && proc.status) ? Number(proc.status) : 1;
}

function run(argv = process.argv.slice(2)) {
  const sub = normalizeSubcommand(argv[0]);
  const rest = argv.slice(1);
  const args = ['rust50-migration-program', sub].concat(rest);
  return runProtheusOpsBridge(args);
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run, normalizeSubcommand };
