#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::run-protheus-ops (authoritative shared operator bridge).

module.exports = require('../../../../adapters/runtime/run_protheus_ops.ts');

if (require.main === module) {
  process.exit(module.exports.runProtheusOps(process.argv.slice(2)));
}
