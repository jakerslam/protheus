#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops::protheus-control-plane (authoritative)
// Thin TypeScript launcher wrapper only.

const { runProtheusOps } = require('./run_protheus_ops.ts');

function run(argv = process.argv.slice(2)) {
  const args = Array.isArray(argv)
    ? argv.map((token) => String(token || '').trim()).filter(Boolean)
    : [];
  const sub = argv[0] ? String(argv[0]).toLowerCase() : 'status';
  const mapped =
    sub === 'status' || sub === 'health'
      ? ['status'].concat(args.slice(1))
      : ['run'].concat(args);
  return runProtheusOps(['protheus-control-plane', ...mapped], {
    env: {
      PROTHEUS_OPS_USE_PREBUILT: '0',
      PROTHEUS_OPS_LOCAL_TIMEOUT_MS: '120000',
    },
    unknownDomainFallback: false,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run };
