#!/usr/bin/env node
'use strict';

// App ownership: apps/examples/singularity-seed-demo (singularity seed example app)

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');

function isFile(filePath) {
  try {
    return fs.statSync(filePath).isFile();
  } catch {
    return false;
  }
}

function resolveBinary() {
  const explicit = String(process.env.PROTHEUS_SINGULARITY_SEED_BINARY || '').trim();
  if (explicit && isFile(explicit)) return explicit;

  const target = path.join(
    ROOT,
    'target',
    'debug',
    process.platform === 'win32' ? 'singularity_seed_core.exe' : 'singularity_seed_core'
  );
  if (isFile(target)) return target;
  return '';
}

function main() {
  const bin = resolveBinary();
  const manifestPath = path.join(ROOT, 'core', 'layer0', 'singularity_seed', 'Cargo.toml');
  const cmd = bin
    ? [bin, 'demo']
    : ['cargo', 'run', '--quiet', '--manifest-path', manifestPath, '--bin', 'singularity_seed_core', '--', 'demo'];

  const proc = spawnSync(cmd[0], cmd.slice(1), {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['pipe', 'pipe', 'pipe'],
    env: process.env,
  });

  if (proc.stdout) process.stdout.write(proc.stdout);
  if (proc.stderr) process.stderr.write(proc.stderr);
  process.exit(Number.isFinite(proc.status) ? proc.status : 1);
}

main();
