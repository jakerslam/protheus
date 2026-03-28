#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const path = require('path');
const { runProtheusOps } = require('../ops/run_protheus_ops.js');

function run(argv = process.argv.slice(2)) {
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

const entryFile = String((require.main && require.main.filename) || process.argv[1] || '');
if (require.main === module || entryFile.endsWith(`${path.sep}assimilate.js`)) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run };
