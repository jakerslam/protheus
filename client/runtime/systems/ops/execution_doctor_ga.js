#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'execution_doctor_ga', 'RUNTIME-SYSTEMS-OPS-EXECUTION_DOCTOR_GA');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
