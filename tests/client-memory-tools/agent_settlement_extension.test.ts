#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Legacy JS test surface retired; authoritative checks are Rust-side.
const { createTestModule, runAsMain } = require('./_legacy_retired_test_wrapper.ts');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');
const mod = createTestModule(__dirname, 'agent_settlement_extension.test', 'MEMORY-TEST-AGENT_SETTLEMENT_EXTENSION.TEST');
assertNoPlaceholderOrPromptLeak(mod, 'agent_settlement_extension_test');
assertStableToolingEnvelope({ status: 'ok', module: 'agent_settlement_extension' }, 'agent_settlement_extension_test');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
