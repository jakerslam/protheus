#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops::unknown-command-guard (authoritative recovery guidance surface).

const mod = require('../../../../adapters/runtime/protheus_cli_modules.ts').protheusUnknownGuard;

function normalizeArgs(argv = process.argv.slice(2)) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function run(argv = process.argv.slice(2)) {
  const status = Number(mod.run(normalizeArgs(argv)));
  return Number.isFinite(status) ? status : 1;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  ...mod,
  normalizeArgs,
  run,
};
