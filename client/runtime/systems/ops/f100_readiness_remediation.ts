#!/usr/bin/env node
'use strict';

// Thin runtime wrapper: core logic lives in tests/tooling/scripts/ops.

const path = require('path');
const { invokeTsModuleSync } = require('../../lib/in_process_ts_delegate.ts');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const IMPLEMENTATION = path.join(
  ROOT,
  'tests',
  'tooling',
  'scripts',
  'ops',
  'f100_readiness_remediation_impl.ts'
);

function run() {
  const outcome = invokeTsModuleSync(IMPLEMENTATION, {
    cwd: ROOT,
    exportName: 'run',
    teeStdout: true,
    teeStderr: true,
  });
  return typeof outcome.status === 'number' ? outcome.status : 1;
}

if (require.main === module) {
  process.exit(run());
}

module.exports = {
  run,
};
