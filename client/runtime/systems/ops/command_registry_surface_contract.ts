#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: tests/tooling/scripts/ops/command_registry_surface_contract.ts (authoritative operator utility); this file is a thin CLI bridge.

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
  'command_registry_surface_contract.ts',
);

installTsRequireHook();
const impl = require(target);

function main(argv = process.argv.slice(2)) {
  return impl.main(argv);
}

if (require.main === module) {
  process.exit(main(process.argv.slice(2)));
}

module.exports = {
  ...impl,
  main,
};
