#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::run-infring-ops (authoritative shared operator bridge).
const impl = require('../../../../adapters/runtime/run_infring_ops.ts');
process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '1';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';

function normalizeArgs(argv = process.argv.slice(2)) {
  const tokens = Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
  return tokens.length ? tokens : ['status'];
}

function run(argv = process.argv.slice(2), options = {}) {
  const status = Number(impl.runInfringOps(normalizeArgs(argv), options));
  return Number.isFinite(status) ? status : 1;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  ...impl,
  normalizeArgs,
  run,
};
