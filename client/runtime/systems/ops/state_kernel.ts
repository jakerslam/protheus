#!/usr/bin/env node
'use strict';

const legacy = require('../../lib/legacy_retired_wrapper.ts');
const mod = legacy.createLegacyRetiredModuleForFile(__filename);

function normalizeArgs(argv = process.argv.slice(2)) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function run(argv = process.argv.slice(2)) {
  return mod.run(normalizeArgs(argv));
}

function main(argv = process.argv.slice(2)) {
  const out = run(argv);
  const status = Number(out && out.status);
  return Number.isFinite(status) ? status : 1;
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
