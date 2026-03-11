#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/ops + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.ts');
const mod = createLegacyRetiredModule(
  __dirname,
  'rust50_sprint1_batch',
  'RUNTIME-OPS-RUST50-SPRINT1-BATCH'
);
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
