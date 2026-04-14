#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::run-protheus-ops (authoritative shared operator bridge).
const impl = require('../../../../adapters/runtime/run_protheus_ops.ts');

function normalizeArgs(argv = []) {
  const list = Array.isArray(argv) ? argv.map((arg) => String(arg || '').trim()).filter(Boolean) : [];
  if (list.length === 0) return ['status'];
  return list;
}

function run(argv = process.argv.slice(2), options = {}) {
  return impl.runProtheusOps(normalizeArgs(argv), options);
}

if (require.main === module) {
  const exitCode = run(process.argv.slice(2));
  process.exit(Number.isFinite(Number(exitCode)) ? Number(exitCode) : 1);
}

module.exports = {
  ...impl,
  run
};
