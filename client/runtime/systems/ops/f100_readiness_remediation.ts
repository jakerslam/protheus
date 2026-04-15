#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: tests/tooling/scripts/ops/f100_readiness_remediation_impl.ts (authoritative operator utility); this file is a thin CLI bridge.

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
  'f100_readiness_remediation_impl.ts',
);

installTsRequireHook();
const impl = require(target);

function normalizeArgs(argv = process.argv.slice(2)) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function run(argv = process.argv.slice(2)) {
  try {
    const status = Number(impl.run(normalizeArgs(argv)));
    return Number.isFinite(status) ? status : 1;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error || 'unknown_error');
    process.stderr.write(`[infring f100] remediation bridge failed: ${message}\n`);
    return 1;
  }
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  ...impl,
  normalizeArgs,
  run,
};
