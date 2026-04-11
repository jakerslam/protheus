#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: tests/tooling/scripts/ops/workspace_command_index.ts (authoritative operator utility); this file is a thin CLI bridge.

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
  'workspace_command_index.ts',
);

installTsRequireHook();
const impl = require(target);

function run(argv = process.argv.slice(2), packageJsonPath = 'package.json', profilePath = 'client/runtime/config/workspace_command_profiles.json') {
  return impl.run(argv, packageJsonPath, profilePath);
}

module.exports = {
  ...impl,
  run,
};

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}
