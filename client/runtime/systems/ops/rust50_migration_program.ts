#!/usr/bin/env node
'use strict';

// Thin bridge to core authority: rust50-migration-program domain.
const { runProtheusOps } = require('./run_protheus_ops.js');

function normalizeSubcommand(raw) {
  const sub = String(raw || 'status').trim().toLowerCase();
  if (!sub) return 'status';
  return sub;
}

function run(argv = process.argv.slice(2)) {
  const sub = normalizeSubcommand(argv[0]);
  const rest = argv.slice(1);
  return runProtheusOps(['rust50-migration-program', sub, ...rest], {
    unknownDomainFallback: true,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run, normalizeSubcommand };
