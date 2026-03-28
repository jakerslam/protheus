#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops::dashboard_ui (authoritative)
// Thin client wrapper only: delegates all dashboard authority to Rust core.

const { runProtheusOps } = require('../ops/run_protheus_ops.js');

function normalizeArgs(argv = process.argv.slice(2)) {
  const args = Array.isArray(argv)
    ? argv.map((token) => String(token || '').trim()).filter(Boolean)
    : [];
  return args;
}

function run(argv = process.argv.slice(2)) {
  const passArgs = normalizeArgs(argv);
  return runProtheusOps(['dashboard-ui', ...passArgs], {
    unknownDomainFallback: true,
    env: {
      PROTHEUS_OPS_USE_PREBUILT: process.env.PROTHEUS_OPS_USE_PREBUILT || '0',
      PROTHEUS_OPS_LOCAL_TIMEOUT_MS: process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000',
    },
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
  normalizeArgs,
};
