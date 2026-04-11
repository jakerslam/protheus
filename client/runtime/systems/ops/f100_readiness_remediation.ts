#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: tests/tooling/scripts/ops/f100_readiness_remediation_impl.ts (authoritative operator utility); this file is a thin CLI bridge.

const path = require('path');
const { installTsRequireHook } = require('../../lib/ts_bootstrap.ts');

const target = path.resolve(
  __dirname,
  '..',
  '..',
  '..',
  '..',
  'tests',
  'tooling',
  'scripts',
  'ops',
  'f100_readiness_remediation_impl.ts',
);

installTsRequireHook();
const impl = require(target);

function run(argv = process.argv.slice(2)) {
  return Number(impl.run(Array.isArray(argv) ? argv : [])) || 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  ...impl,
  run,
};
