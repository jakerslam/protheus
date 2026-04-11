#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::run-protheus-ops (authoritative shared operator bridge).
const impl = require('../../../../adapters/runtime/run_protheus_ops.ts');

if (require.main === module) {
  const exitCode = impl.runProtheusOps(process.argv.slice(2));
  process.exit(Number.isFinite(Number(exitCode)) ? Number(exitCode) : 1);
}

module.exports = impl;
