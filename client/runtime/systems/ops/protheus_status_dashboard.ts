#!/usr/bin/env node
'use strict';

// Thin compatibility lane: dashboard authority is Rust-core (`daemon-control`).
const { runProtheusOps } = require('./run_protheus_ops.ts');

function hasWebFlag(argv) {
  return Array.isArray(argv) && argv.some((arg) => arg === '--web' || arg === 'web');
}

function stripDashboardCompatFlags(argv) {
  return (Array.isArray(argv) ? argv : []).filter(
    (arg) =>
      arg !== '--dashboard' &&
      arg !== 'dashboard' &&
      arg !== '--web' &&
      arg !== 'web'
  );
}

function runDashboardUi(argv) {
  const forward = stripDashboardCompatFlags(argv);
  return runProtheusOps(
    ['daemon-control', 'start', ...forward],
    { unknownDomainFallback: true }
  );
}

function run(argv = process.argv.slice(2)) {
  if (hasWebFlag(argv)) {
    return runDashboardUi(argv);
  }
  const passthrough = argv.length
    ? argv
    : ['daemon-control', 'status'];
  return runProtheusOps(passthrough, { unknownDomainFallback: true });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run, runDashboardUi };
