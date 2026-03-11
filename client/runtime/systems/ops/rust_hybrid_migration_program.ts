#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/ops + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.ts');
const mod = createLegacyRetiredModule(
  __dirname,
  'rust_hybrid_migration_program',
  'RUNTIME-OPS-RUST-HYBRID-MIGRATION-PROGRAM'
);
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
