#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Legacy JS test surface retired; authoritative checks are Rust-side.
const { createTestModule, runAsMain } = require('./_legacy_retired_test_wrapper.ts');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');
const mod = createTestModule(__dirname, 'assimilation_usage_integration.test', 'MEMORY-TEST-ASSIMILATION_USAGE_INTEGRATION.TEST');
assertNoPlaceholderOrPromptLeak(mod, 'assimilation_usage_integration_test');
assertStableToolingEnvelope({ status: 'ok', module: 'assimilation_usage_integration' }, 'assimilation_usage_integration_test');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
