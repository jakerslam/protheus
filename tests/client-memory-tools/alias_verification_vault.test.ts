#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Legacy JS test surface retired; authoritative checks are Rust-side.
const { createTestModule, runAsMain } = require('./_legacy_retired_test_wrapper.ts');
const mod = createTestModule(__dirname, 'alias_verification_vault.test', 'MEMORY-TEST-ALIAS_VERIFICATION_VAULT.TEST');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
