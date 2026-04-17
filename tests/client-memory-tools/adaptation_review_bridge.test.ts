#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Legacy JS test surface retired; authoritative checks are Rust-side.
const { createTestModule, runAsMain } = require('./_legacy_retired_test_wrapper.ts');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');
const mod = createTestModule(__dirname, 'adaptation_review_bridge.test', 'MEMORY-TEST-ADAPTATION_REVIEW_BRIDGE.TEST');
assertNoPlaceholderOrPromptLeak(mod, 'adaptation_review_bridge_test');
assertStableToolingEnvelope({ status: 'ok', module: 'adaptation_review_bridge' }, 'adaptation_review_bridge_test');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
