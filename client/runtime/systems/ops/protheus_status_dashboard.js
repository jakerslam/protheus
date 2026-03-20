#!/usr/bin/env node
'use strict';

// Compatibility wrapper: dashboard requests route to control-plane status.

const path = require('path');
const { spawnSync } = require('child_process');
const { run } = require('./protheus_control_plane.js');

function hasWebFlag(args) {
  return args.includes('--web') || args.includes('web');
}

function launchWebDashboard(args) {
  const entrypoint = path.resolve(__dirname, '..', '..', 'lib', 'ts_entrypoint.ts');
  const target = path.resolve(__dirname, '..', 'ui', 'infring_dashboard.ts');
  const forward = args.filter((arg) => arg !== '--dashboard' && arg !== 'dashboard' && arg !== '--web');
  const proc = spawnSync(process.execPath, [entrypoint, target, 'serve', ...forward], {
    cwd: path.resolve(__dirname, '..', '..', '..', '..'),
    stdio: 'inherit',
    env: process.env,
  });
  if (proc.error) {
    process.stderr.write(`infring_dashboard_launch_error:${String(proc.error.message || proc.error)}\n`);
    return 1;
  }
  return Number.isFinite(proc.status) ? proc.status : 1;
}

if (require.main === module) {
  const args = process.argv.slice(2);
  const exitCode = hasWebFlag(args)
    ? launchWebDashboard(args)
    : run(['status'].concat(args));
  process.exit(exitCode);
}

module.exports = { run, launchWebDashboard };
