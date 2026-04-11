#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops::daemon-control (authoritative dashboard/operator status route).

const mod = require('../../../../adapters/runtime/protheus_cli_modules.ts').protheusStatusDashboard;

if (require.main === module) {
  process.exit(mod.run(process.argv.slice(2)));
}

module.exports = mod;
