#!/usr/bin/env node
'use strict';

// App ownership: apps/examples/dictionary-demo (toolkit example app)

const path = require('path');
const { spawnSync } = require('child_process');
const ROOT = path.resolve(__dirname, '..', '..', '..');

function runToolkit(args, opts) {
  const options = opts || {};
  const proc = spawnSync('cargo', [
    'run', '--quiet', '-p', 'protheus-ops-core', '--bin', 'protheus-ops', '--',
    'protheusctl', 'toolkit', ...args
  ], {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: 'pipe',
    input: options.input || undefined,
    env: process.env
  });
  if (proc.stdout) process.stdout.write(proc.stdout);
  if (proc.stderr) process.stderr.write(proc.stderr);
  process.exit(Number.isFinite(proc.status) ? proc.status : 1);
}

runToolkit(['dictionary', 'term', 'Binary Blobs']);
