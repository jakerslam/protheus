#!/usr/bin/env node
'use strict';

// Layer ownership: tests/tooling/scripts/ops/dr_gameday.ts (authoritative operator utility)
// Thin TypeScript wrapper only.

const path = require('path');
const { installTsRequireHook } = require('../../lib/ts_bootstrap.ts');

function run(argv = process.argv.slice(2)) {
  const target = path.resolve(__dirname, '..', '..', '..', '..', 'tests', 'tooling', 'scripts', 'ops', 'dr_gameday.ts');
  installTsRequireHook();
  const { run: targetRun } = require(target);
  return Number(targetRun(Array.isArray(argv) ? argv : [])) || 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run };
