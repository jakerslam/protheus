#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops::command-list-kernel (authoritative)
// Thin TypeScript launcher wrapper only.
const { runInfringOps } = require('./run_infring_ops.ts');
const DEFAULT_ARGS = ['--mode=help'];

function normalizeArgs(argv = process.argv.slice(2)) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function resolveArgs(argv = process.argv.slice(2)) {
  const args = normalizeArgs(argv);
  return args.length > 0 ? args : DEFAULT_ARGS.slice(0);
}

function run(argv = process.argv.slice(2)): number {
  const args = resolveArgs(argv);
  return runInfringOps(['command-list-kernel', ...args], {
    env: {
      INFRING_OPS_USE_PREBUILT: process.env.INFRING_OPS_USE_PREBUILT || '0',
      INFRING_OPS_LOCAL_TIMEOUT_MS: process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000'
    },
    unknownDomainFallback: false
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  DEFAULT_ARGS,
  normalizeArgs,
  resolveArgs,
  run,
};
