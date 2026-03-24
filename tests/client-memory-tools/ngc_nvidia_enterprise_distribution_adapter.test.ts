#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Legacy JS test surface retired; authoritative checks are Rust-side.
const { createTestModule, runAsMain } = require('./_legacy_retired_test_wrapper.ts');
const mod = createTestModule(__dirname, 'ngc_nvidia_enterprise_distribution_adapter.test', 'MEMORY-TEST-NGC_NVIDIA_ENTERPRISE_DISTRIBUTION_ADAPTER.TEST');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
