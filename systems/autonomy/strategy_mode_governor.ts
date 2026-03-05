#!/usr/bin/env node
'use strict';
export {};

/**
 * Rust cutover wrapper for strategy_mode_governor.
 *
 * - Preserves CLI path (`node systems/autonomy/strategy_mode_governor.js ...`)
 * - Delegates CLI execution to Rust domain in `crates/ops`
 * - Re-exports legacy helper functions for existing JS callers
 */

const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const legacy = require('./strategy_mode_governor_legacy.js');

function runRustCli() {
  const args = process.argv.slice(2);
  const cargoArgs = [
    'run',
    '--quiet',
    '--manifest-path',
    'crates/ops/Cargo.toml',
    '--bin',
    'protheus-ops',
    '--',
    'strategy-mode-governor',
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
  runRustCli();
}

module.exports = legacy;
