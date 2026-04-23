#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative version/update lane)
// Thin TypeScript wrapper only.

const { runInfringOps } = require('./run_infring_ops.ts');
process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '1';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';

function normalizeArgs(argv = process.argv.slice(2)) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function resolveArgs(argv = process.argv.slice(2)) {
  return ['version-cli', ...normalizeArgs(argv)];
}

function run(argv = process.argv.slice(2)) {
  const status = Number(runInfringOps(resolveArgs(argv), {
    unknownDomainFallback: false
  }));
  return Number.isFinite(status) ? status : 1;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  normalizeArgs,
  resolveArgs,
  run,
};
