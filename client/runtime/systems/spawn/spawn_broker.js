#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/ops::spawn_broker (authoritative)
// Thin client wrapper routes all spawn-broker authority through core Rust lane.
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

function usage() {
  process.stdout.write('Usage: spawn_broker.js <status|request|release> [flags]\n');
}

function mapArgs(argv = []) {
  const args = Array.isArray(argv) ? argv.slice() : [];
  const cmd = String(args[0] || '').trim().toLowerCase();
  if (!cmd || cmd === 'help' || cmd === '--help' || cmd === '-h') {
    return { help: true, coreArgs: [] };
  }
  if (cmd === 'status' || cmd === 'request' || cmd === 'release') {
    return { help: false, coreArgs: args };
  }
  return { help: false, coreArgs: args };
}

function repoRoot() {
  let dir = path.resolve(__dirname);
  while (true) {
    const marker = path.join(dir, 'core', 'layer0', 'ops', 'Cargo.toml');
    if (fs.existsSync(marker)) return dir;
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return path.resolve(__dirname, '..', '..', '..', '..');
}

function runCore(coreArgs = []) {
  const root = repoRoot();
  const explicitBin = String(process.env.PROTHEUS_OPS_BIN || '').trim();
  const timeoutMs = Math.max(
    1000,
    Math.min(300000, Number(process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || 120000))
  );

  let command = '';
  let args = [];
  if (explicitBin) {
    command = explicitBin;
    args = ['spawn-broker', ...(Array.isArray(coreArgs) ? coreArgs : [])];
  } else {
    command = 'cargo';
    args = [
      'run',
      '--quiet',
      '--manifest-path',
      'core/layer0/ops/Cargo.toml',
      '--bin',
      'protheus-ops',
      '--',
      'spawn-broker',
      ...(Array.isArray(coreArgs) ? coreArgs : [])
    ];
  }

  const out = spawnSync(command, args, {
    cwd: root,
    encoding: 'utf8',
    env: { ...process.env, PROTHEUS_NODE_BINARY: process.execPath || 'node' },
    timeout: timeoutMs,
    maxBuffer: 1024 * 1024 * 4
  });

  return {
    status: Number.isFinite(out.status) ? Number(out.status) : 1,
    stdout: String(out.stdout || ''),
    stderr: `${String(out.stderr || '')}${out.error ? `\n${String(out.error.message || out.error)}` : ''}`
  };
}

function run(argv = []) {
  const mapped = mapArgs(argv);
  if (mapped.help) {
    usage();
    return 0;
  }
  const out = runCore(mapped.coreArgs);
  if (out.stdout) process.stdout.write(out.stdout);
  if (out.stderr) process.stderr.write(out.stderr);
  return Number.isFinite(out.status) ? Number(out.status) : 1;
}

if (require.main === module) {
  const code = run(process.argv.slice(2));
  process.exit(code);
}

module.exports = {
  run
};
