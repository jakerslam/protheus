#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'spine_kernel_budget_check', 'RUNTIME-SYSTEMS-OPS-SPINE_KERNEL_BUDGET_CHECK');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
