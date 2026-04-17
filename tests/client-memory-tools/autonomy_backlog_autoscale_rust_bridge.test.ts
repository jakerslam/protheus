#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Legacy JS test surface retired; authoritative checks are Rust-side.
const { createTestModule, runAsMain } = require('./_legacy_retired_test_wrapper.ts');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');
const mod = createTestModule(__dirname, 'autonomy_backlog_autoscale_rust_bridge.test', 'MEMORY-TEST-AUTONOMY_BACKLOG_AUTOSCALE_RUST_BRIDGE.TEST');
assertNoPlaceholderOrPromptLeak(mod, 'autonomy_backlog_autoscale_rust_bridge_test');
assertStableToolingEnvelope({ status: 'ok', module: 'autonomy_backlog_autoscale_rust_bridge' }, 'autonomy_backlog_autoscale_rust_bridge_test');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
