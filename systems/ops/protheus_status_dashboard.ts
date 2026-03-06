#!/usr/bin/env node
'use strict';

/**
 * Rust-authoritative status dashboard wrapper.
 * TS remains a thin CLI surface only.
 */

const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');

function runRustStatusDashboard(args = []) {
  const cargoArgs = [
    'run',
    '--quiet',
    '--manifest-path',
    'crates/ops/Cargo.toml',
    '--bin',
    'protheus-ops',
    '--',
    'status',
    '--dashboard',
    ...args
  ];
  const out = spawnSync('cargo', cargoArgs, {
    cwd: ROOT,
    encoding: 'utf8',
    env: {
      ...process.env,
      PROTHEUS_NODE_BINARY: process.execPath || 'node'
    }
  });

  const status = Number.isFinite(out.status) ? out.status : 1;
  return {
    ok: status === 0,
    status,
    stdout: out.stdout || '',
    stderr: out.stderr || ''
  };
}

if (require.main === module) {
  const out = runRustStatusDashboard(process.argv.slice(2));
  if (out.stdout) process.stdout.write(out.stdout);
  if (out.stderr) process.stderr.write(out.stderr);
  process.exit(out.status);
}

module.exports = {
  runRustStatusDashboard
};
