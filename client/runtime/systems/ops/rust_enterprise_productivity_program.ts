#!/usr/bin/env node
'use strict';

// Thin bridge to core authority: rust-enterprise-productivity-program domain.

const { runProtheusOps } = require('./run_protheus_ops.js');

function normalizeSubcommand(raw) {
  const sub = String(raw || 'status').trim().toLowerCase();
  if (!sub) return 'status';
  return sub;
}

function run(argv = process.argv.slice(2)) {
  const sub = normalizeSubcommand(argv[0]);
  const rest = argv.slice(1);
  const args = ['rust-enterprise-productivity-program', sub].concat(rest);
  return runProtheusOps(args, { unknownDomainFallback: true });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run, normalizeSubcommand };
