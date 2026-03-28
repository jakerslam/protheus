#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative).
// JS shim remains for stable CLI entrypoint compatibility only.
const { runProtheusOps } = require('../ops/run_protheus_ops.js');

function run(argv) {
  const args = Array.isArray(argv)
    ? argv.map((token) => String(token || '').trim()).filter(Boolean)
    : [];
  return runProtheusOps(['assimilate-kernel', ...args], {
    env: {
      PROTHEUS_OPS_USE_PREBUILT: '0',
      PROTHEUS_OPS_LOCAL_TIMEOUT_MS: '120000'
    },
    unknownDomainFallback: false
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run };
