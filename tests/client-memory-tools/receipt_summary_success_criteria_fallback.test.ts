#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Legacy JS test surface retired; authoritative checks are Rust-side.
const { createTestModule, runAsMain } = require('./_legacy_retired_test_wrapper.ts');
const mod = createTestModule(__dirname, 'receipt_summary_success_criteria_fallback.test', 'MEMORY-TEST-RECEIPT_SUMMARY_SUCCESS_CRITERIA_FALLBACK.TEST');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
