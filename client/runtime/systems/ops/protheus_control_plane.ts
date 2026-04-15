#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops::protheus-control-plane (authoritative domain route).

const mod = require('../../../../adapters/runtime/protheus_cli_modules.ts').protheusControlPlane;
process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '1';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '600000';

function normalizeArgs(argv = process.argv.slice(2)) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function run(argv = process.argv.slice(2)) {
  const status = Number(mod.run(normalizeArgs(argv)));
  return Number.isFinite(status) ? status : 1;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  ...mod,
  normalizeArgs,
  run,
};
