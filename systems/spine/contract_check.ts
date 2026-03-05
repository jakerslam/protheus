#!/usr/bin/env node
'use strict';
export {};

/**
 * Rust cutover wrapper for contract_check.
 *
 * Keeps the existing entrypoint while routing execution through the
 * `protheus-ops` Rust domain.
 */

const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');

function main() {
  const args = process.argv.slice(2);
  const cargoArgs = [
    'run',
    '--quiet',
    '--manifest-path',
    'crates/ops/Cargo.toml',
    '--',
    'contract-check',
    ...args
  ];

  const run = spawnSync('cargo', cargoArgs, {
    cwd: ROOT,
    encoding: 'utf8',
    env: {
      ...process.env,
      PROTHEUS_NODE_BINARY: process.execPath || 'node'
    }
  });

  if (run.stdout) process.stdout.write(run.stdout);
  if (run.stderr) process.stderr.write(run.stderr);
  process.exit(Number.isFinite(run.status) ? run.status : 1);
}

if (require.main === module) {
  main();
}
