#!/usr/bin/env node
'use strict';

// Thin bridge to core authority: venom-containment-layer domain.

const { runProtheusOps } = require('../ops/run_protheus_ops.ts');

function normalizeSubcommand(raw) {
  const sub = String(raw || 'status').trim().toLowerCase();
  if (!sub) return 'status';
  if (sub === 'evolve') return 'evaluate';
  return sub;
}

function run(argv = process.argv.slice(2)) {
  const sub = normalizeSubcommand(argv[0]);
  const rest = argv.slice(1);
  const args = ['venom-containment-layer', sub].concat(rest);
  return runProtheusOps(args, { unknownDomainFallback: true });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run, normalizeSubcommand };
