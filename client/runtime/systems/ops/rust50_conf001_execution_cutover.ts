#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/ops + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.ts');
const mod = createLegacyRetiredModule(
  __dirname,
  'rust50_conf001_execution_cutover',
  'RUNTIME-OPS-RUST50-CONF001-EXECUTION-CUTOVER'
);
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
