#!/usr/bin/env node
'use strict';

// Client compatibility shim only.
const impl = require('../../../../adapters/runtime/run_protheus_ops.ts');

if (require.main === module) {
  const exitCode = impl.runProtheusOps(process.argv.slice(2));
  process.exit(Number.isFinite(Number(exitCode)) ? Number(exitCode) : 1);
}

module.exports = impl;
