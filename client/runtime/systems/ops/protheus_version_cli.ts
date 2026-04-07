#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative version/update lane)
// Thin TypeScript wrapper only.

const { runProtheusOps } = require('./run_protheus_ops.ts');

function run(args = process.argv.slice(2)) {
  const passArgs = Array.isArray(args) ? args : [];
  return runProtheusOps(['version-cli', ...passArgs], {
    unknownDomainFallback: false
  });
}

if (require.main === module) {
  const status = run(process.argv.slice(2));
  process.exit(Number.isFinite(Number(status)) ? Number(status) : 1);
}

module.exports = { run };
