#!/usr/bin/env node
'use strict';

// Compatibility lane for `protheus-ops status --dashboard`.

const path = require('path');
const { spawnSync } = require('child_process');
const { runProtheusOps } = require('./run_protheus_ops.ts');

const DASHBOARD_BOOT_MAX_ATTEMPTS = 3;
const DASHBOARD_BOOT_BACKOFF_MS = [300, 900];

function sleepMs(ms) {
  const waitMs = Math.max(0, Number(ms || 0));
  if (!waitMs) return;
  const sab = new SharedArrayBuffer(4);
  const arr = new Int32Array(sab);
  Atomics.wait(arr, 0, 0, waitMs);
}

function hasWebFlag(argv) {
  return Array.isArray(argv) && argv.some((arg) => arg === '--web' || arg === 'web');
}

function runDashboardUi(argv) {
  const entrypoint = path.resolve(__dirname, '..', '..', 'lib', 'ts_entrypoint.ts');
  const uiTarget = path.resolve(__dirname, '..', 'ui', 'infring_dashboard.ts');
  const forward = argv.filter((arg) => arg !== '--dashboard' && arg !== 'dashboard' && arg !== '--web');
  const root = path.resolve(__dirname, '..', '..', '..', '..');
  let finalStatus = 1;

  for (let attempt = 1; attempt <= DASHBOARD_BOOT_MAX_ATTEMPTS; attempt += 1) {
    const proc = spawnSync(process.execPath, [entrypoint, uiTarget, 'serve', ...forward], {
      stdio: 'inherit',
      cwd: root,
      env: process.env,
    });
    if (proc.error) {
      finalStatus = 1;
      console.error(`infring_dashboard_launch_error:${String(proc.error.message || proc.error)}`);
    } else {
      finalStatus = Number.isFinite(proc.status) ? proc.status : 1;
    }
    if (finalStatus === 0) return 0;
    if (attempt < DASHBOARD_BOOT_MAX_ATTEMPTS) {
      const backoffMs = DASHBOARD_BOOT_BACKOFF_MS[Math.max(0, Math.min(DASHBOARD_BOOT_BACKOFF_MS.length - 1, attempt - 1))] || 1200;
      console.error(`infring_dashboard_restart_attempt:${attempt + 1}/${DASHBOARD_BOOT_MAX_ATTEMPTS}`);
      sleepMs(backoffMs);
    }
  }
  return finalStatus;
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
