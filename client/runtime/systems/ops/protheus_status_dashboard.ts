#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops::daemon-control (authoritative dashboard/operator status route).

const mod = require('../../../../adapters/runtime/protheus_cli_modules.ts').protheusStatusDashboard;
process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '1';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '600000';

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
