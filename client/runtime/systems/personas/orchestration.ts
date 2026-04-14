#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: surface/orchestration (persona orchestration coordination); this file is a thin CLI bridge.

const impl = require('../../../../surface/orchestration/scripts/personas_orchestration.ts');

function run(args = process.argv.slice(2)) {
  return impl.run(args);
}

function normalizeExitCode(result) {
  if (typeof result === 'number' && Number.isFinite(result)) return result;
  if (result && typeof result === 'object') {
    if (typeof result.exitCode === 'number' && Number.isFinite(result.exitCode)) return result.exitCode;
    if (typeof result.exit_code === 'number' && Number.isFinite(result.exit_code)) return result.exit_code;
    if (typeof result.ok === 'boolean') return result.ok ? 0 : 1;
  }
  return 0;
}

if (require.main === module) {
  process.exit(normalizeExitCode(run(process.argv.slice(2))));
}

module.exports = {
  ...impl,
  run
};
