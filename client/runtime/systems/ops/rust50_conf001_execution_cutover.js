#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'rust50_conf001_execution_cutover', 'RUNTIME-SYSTEMS-OPS-RUST50_CONF001_EXECUTION_CUTOVER');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
