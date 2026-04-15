#!/usr/bin/env node
'use strict';

// Layer ownership: tests/tooling/scripts/ops/dr_gameday.ts (authoritative operator utility)
// Thin TypeScript wrapper only.

const path = require('path');
const { installTsRequireHook } = require('../../lib/ts_bootstrap.ts');

function normalizeArgs(argv = process.argv.slice(2)) {
  const passArgs = Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
  if (!passArgs.length) {
    return ['gate'];
  }
  return passArgs.map((token, index) => (index === 0 && token === 'run' ? 'gate' : token));
}

function resolveTarget() {
  return path.resolve(__dirname, '..', '..', '..', '..', 'tests', 'tooling', 'scripts', 'ops', 'dr_gameday.ts');
}

function run(argv = process.argv.slice(2)) {
  const passArgs = normalizeArgs(argv);
  const target = resolveTarget();
  installTsRequireHook();
  const { run: targetRun } = require(target);
  return Number(targetRun(passArgs)) || 0;
}

function main(argv = process.argv.slice(2)) {
  const status = Number(run(argv));
  return Number.isFinite(status) ? status : 1;
}

if (require.main === module) {
  process.exit(main(process.argv.slice(2)));
}

module.exports = {
  normalizeArgs,
  resolveTarget,
  run,
  main,
};
