#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops::backlog-github-sync (authoritative domain route).

const mod = require('../../../../adapters/runtime/infring_cli_modules.ts').backlogGithubSync;

function normalizeCompatArgv(argv = process.argv.slice(2)) {
  const args = Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
  if (!args.length) return args;
  const aliases = {
    mirror: 'sync',
    triage: 'check'
  };
  const first = String(args[0] || '').toLowerCase();
  if (Object.prototype.hasOwnProperty.call(aliases, first)) {
    args[0] = aliases[first];
  }
  return args;
}

function main(argv = process.argv.slice(2)) {
  process.exit(mod.run(normalizeCompatArgv(argv)));
}

if (require.main === module) {
  main(process.argv.slice(2));
}

module.exports = {
  ...mod,
  main,
  normalizeCompatArgv
};
