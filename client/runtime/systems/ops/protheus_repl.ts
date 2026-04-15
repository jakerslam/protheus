#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops::command-list-kernel (authoritative operator help surface).

const base = require('../../../../adapters/runtime/protheus_cli_modules.ts').protheusRepl;

function normalizeArgs(argv = process.argv.slice(2)) {
  return Array.isArray(argv)
    ? argv.map((token) => String(token || '').trim()).filter(Boolean)
    : [];
}

function run(argv = process.argv.slice(2)) {
  return base.run(normalizeArgs(argv));
}

function runCli(argv = process.argv.slice(2)) {
  const status = Number(run(argv));
  return Number.isFinite(status) ? status : 1;
}

if (require.main === module) {
  process.exit(runCli(process.argv.slice(2)));
}

module.exports = {
  ...base,
  normalizeArgs,
  run,
  runCli,
};
