#!/usr/bin/env tsx
// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops::contract-check (authoritative contract validation route).

const mod = require('../../../../adapters/runtime/protheus_cli_modules.ts').contractCheck;

if (require.main === module) {
  process.exit(mod.run(process.argv.slice(2)));
}

module.exports = mod;
