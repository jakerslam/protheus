#!/usr/bin/env node
'use strict';

// JS shim contract wrapper for `protheus status --dashboard`.
// Behavior intentionally mirrors protheus_status_dashboard.ts.

const path = require('path');
const { spawnSync } = require('child_process');
const { runProtheusOps } = require('./run_protheus_ops.js');

function hasWebFlag(argv) {
  return Array.isArray(argv) && argv.some((arg) => arg === '--web' || arg === 'web');
}

function runDashboardUi(argv) {
  const entrypoint = path.resolve(__dirname, '..', '..', 'lib', 'ts_entrypoint.ts');
  const uiTarget = path.resolve(__dirname, '..', 'ui', 'infring_dashboard.ts');
  const forward = argv.filter((arg) => arg !== '--dashboard' && arg !== 'dashboard' && arg !== '--web');
  const proc = spawnSync(process.execPath, [entrypoint, uiTarget, 'serve', ...forward], {
    stdio: 'inherit',
    cwd: path.resolve(__dirname, '..', '..', '..', '..'),
    env: process.env,
  });
  if (proc.error) {
    console.error(`infring_dashboard_launch_error:${String(proc.error.message || proc.error)}`);
    return 1;
  }
  return Number.isFinite(proc.status) ? proc.status : 1;
}

function run(argv = process.argv.slice(2)) {
  if (hasWebFlag(argv)) {
    return runDashboardUi(argv);
  }
  const passthrough = argv.length ? argv : ['status', '--dashboard'];
  return runProtheusOps(passthrough, { unknownDomainFallback: true });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run, runDashboardUi };
