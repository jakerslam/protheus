#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops::daemon-control (authoritative dashboard/operator status route).

const mod = require('../../../../adapters/runtime/infring_cli_modules.ts').infringStatusDashboard;
process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '1';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '600000';

function normalizeArgs(argv = process.argv.slice(2)) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function run(argv = process.argv.slice(2)) {
  const status = Number(mod.run(normalizeArgs(argv)));
  return Number.isFinite(status) ? status : 1;
}

function runDashboardUi(argv = process.argv.slice(2)) {
  if (typeof mod.runDashboardUi !== 'function') return 1;
  const status = Number(mod.runDashboardUi(normalizeArgs(argv)));
  return Number.isFinite(status) ? status : 1;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  ...mod,
  normalizeArgs,
  run,
  runDashboardUi,
};
