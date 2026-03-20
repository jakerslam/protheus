#!/usr/bin/env tsx
// Compatibility lane for `protheus-ops status --dashboard`.

import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { runProtheusOps } from './run_protheus_ops.js';

function hasWebFlag(argv: string[]): boolean {
  return argv.some((arg) => arg === '--web' || arg === 'web');
}

function runDashboardUi(argv: string[]): number {
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
  return Number.isFinite(proc.status) ? proc.status ?? 1 : 1;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  if (hasWebFlag(argv)) {
    return runDashboardUi(argv);
  }
  const passthrough = argv.length ? argv : ['status', '--dashboard'];
  return runProtheusOps(passthrough, { unknownDomainFallback: true });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}
