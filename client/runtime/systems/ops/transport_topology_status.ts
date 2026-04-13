#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: tests/tooling/scripts/ops/transport_topology_status.ts (authoritative operator utility); this file is a thin CLI bridge.
// Runtime contract markers: resident_ipc_authoritative, process_fallback_effective.

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
  'transport_topology_status.ts',
);

installTsRequireHook();
const impl = require(target);

function run(args = process.argv.slice(2), env = process.env) {
  return impl.run(args, env);
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2), process.env));
}

module.exports = {
  ...impl,
  run,
};
