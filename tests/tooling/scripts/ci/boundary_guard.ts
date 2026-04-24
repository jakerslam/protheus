#!/usr/bin/env tsx

import { spawnSync } from 'node:child_process';
import path from 'node:path';

const root = process.cwd();
const entrypoint = path.resolve(root, 'client/runtime/lib/ts_entrypoint.ts');
const script = path.resolve(root, 'tests/tooling/scripts/ci/cross_layer_import_guard.ts');
const args = process.argv.slice(2);

function hasFlag(name: string): boolean {
  return args.some((arg) => arg === name || arg.startsWith(`${name}=`));
}

const forwarded = [...args];
if (!hasFlag('--out-json')) {
  forwarded.push('--out-json=core/local/artifacts/boundary_guard_current.json');
}
if (!hasFlag('--out-markdown')) {
  forwarded.push('--out-markdown=local/workspace/reports/BOUNDARY_GUARD_CURRENT.md');
}

const proc = spawnSync('node', [entrypoint, script, ...forwarded], {
  cwd: root,
  encoding: 'utf8',
  stdio: ['ignore', 'pipe', 'pipe'],
  maxBuffer: 64 * 1024 * 1024,
});

if (proc.stdout) process.stdout.write(proc.stdout);
if (proc.stderr) process.stderr.write(proc.stderr);
process.exit(proc.status ?? 1);
