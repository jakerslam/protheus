#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops::rust-enterprise-productivity-program (authoritative domain route).

const mod = require('../../../../adapters/runtime/protheus_cli_modules.ts').rustEnterpriseProductivityProgram;

if (require.main === module) {
  process.exit(mod.run(process.argv.slice(2)));
}

module.exports = mod;
