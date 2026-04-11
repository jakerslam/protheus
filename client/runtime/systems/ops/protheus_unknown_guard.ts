#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops::unknown-command-guard (authoritative recovery guidance surface).

const mod = require('../../../../adapters/runtime/protheus_cli_modules.ts').protheusUnknownGuard;

if (require.main === module) {
  process.exit(mod.run(process.argv.slice(2)));
}

module.exports = mod;
