#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Legacy JS test surface retired; authoritative checks are Rust-side.
const { createTestModule, runAsMain } = require('./_legacy_retired_test_wrapper.js');
const mod = createTestModule(__dirname, 'offline_r_analytics_runner.test', 'MEMORY-TEST-OFFLINE_R_ANALYTICS_RUNNER.TEST');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
