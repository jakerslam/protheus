#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops::venom-containment-layer (authoritative domain route).

const mod = require('../../../../adapters/runtime/protheus_cli_modules.ts').venomContainmentLayer;

function normalizeArgs(argv = process.argv.slice(2)) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function run(argv = process.argv.slice(2)) {
  return mod.run(normalizeArgs(argv));
}

function main(argv = process.argv.slice(2)) {
  const out = run(argv);
  const status = Number(out && out.status);
  return Number.isFinite(status) ? status : (Number.isFinite(Number(out)) ? Number(out) : 1);
}

if (require.main === module) {
  process.exit(main(process.argv.slice(2)));
}

module.exports = {
  ...mod,
  normalizeArgs,
  run,
  main,
};
