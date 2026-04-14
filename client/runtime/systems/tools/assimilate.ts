#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const path = require('path');
const { runProtheusOps } = require('../ops/run_protheus_ops.ts');

function normalizeArgs(argv) {
  if (!Array.isArray(argv)) return [];
  return argv.map((token) => String(token || '').trim()).filter(Boolean);
}

function hasTruthyFlag(args, key) {
  return args.some((arg) => arg === `--${key}` || arg === `--${key}=1` || arg === `--${key}=true`);
}

function hasFalseyFlag(args, key) {
  return args.some((arg) => arg === `--${key}=0` || arg === `--${key}=false`);
}

function ensureLocalSimulationCompat(args) {
  if (!Array.isArray(args) || args.length === 0) return [];
  if (hasTruthyFlag(args, 'strict') || hasFalseyFlag(args, 'allow-local-simulation')) {
    return args;
  }
  if (hasTruthyFlag(args, 'allow-local-simulation')) {
    return args;
  }
  const hasTarget = args.some((arg) => !arg.startsWith('-'));
  if (!hasTarget) return args;
  return args.concat('--allow-local-simulation=1');
}

function run(argv = process.argv.slice(2)) {
  const args = ensureLocalSimulationCompat(normalizeArgs(argv));
  const domainArgs = args[0] === 'assimilate-kernel' ? args : ['assimilate-kernel', ...args];
  return runProtheusOps(domainArgs, {
    env: {
      PROTHEUS_OPS_USE_PREBUILT: '0',
      PROTHEUS_OPS_LOCAL_TIMEOUT_MS: '120000'
    },
    unknownDomainFallback: false
  });
}

const entryFile = String((require.main && require.main.filename) || process.argv[1] || '');
if (require.main === module || entryFile.endsWith(`${path.sep}assimilate.ts`)) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  normalizeArgs,
  hasTruthyFlag,
  hasFalseyFlag,
  ensureLocalSimulationCompat,
  run
};
