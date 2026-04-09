#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops::release-semver-contract (authoritative)
// Thin TypeScript launcher wrapper only.

const { runProtheusOps } = require('./run_protheus_ops.ts');

function run(argv = process.argv.slice(2)) {
  const args = Array.isArray(argv)
    ? argv.map((token) => String(token || '').trim()).filter(Boolean)
    : [];
  return runProtheusOps(
    ['release-semver-contract', ...(args.length ? args : ['status'])],
    {
      env: {
        PROTHEUS_OPS_USE_PREBUILT: '0',
        PROTHEUS_OPS_LOCAL_TIMEOUT_MS: '120000',
      },
    }
  );
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run };
