#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Legacy JS test surface retired; authoritative checks are Rust-side.
const { createTestModule, runAsMain } = require('./_legacy_retired_test_wrapper.js');
const mod = createTestModule(__dirname, 'post_launch_migration_readiness.test', 'MEMORY-TEST-POST_LAUNCH_MIGRATION_READINESS.TEST');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
