#!/usr/bin/env node

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops + core/layer1/policy (authoritative)

'use strict';

const path = require('node:path');
const { invokeTsModuleSync } = require('../../lib/in_process_ts_delegate.ts');

const WORKSPACE_ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const GUARD_SCRIPT = path.resolve(WORKSPACE_ROOT, 'tests/tooling/scripts/ci/dependency_boundary_guard.ts');

function run(argv = []) {
  const args = Array.isArray(argv) ? argv.map((v) => String(v)) : [];
  const res = invokeTsModuleSync(GUARD_SCRIPT, {
    argv: args,
    cwd: WORKSPACE_ROOT,
    exportName: 'run',
    teeStdout: true,
    teeStderr: true,
  });
  if (typeof res.status === 'number' && res.status !== 0) {
    process.exit(res.status);
  }
  return { ok: true, delegated_to: 'tests/tooling/scripts/ci/dependency_boundary_guard.ts' };
}

if (require.main === module) {
  run(process.argv.slice(2));
}

module.exports = {
  run,
};
