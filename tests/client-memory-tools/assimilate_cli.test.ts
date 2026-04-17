#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Legacy JS test surface retired; authoritative checks are Rust-side.
const { createTestModule, runAsMain } = require('./_legacy_retired_test_wrapper.ts');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');
const mod = createTestModule(__dirname, 'assimilate_cli.test', 'MEMORY-TEST-ASSIMILATE_CLI.TEST');
assertNoPlaceholderOrPromptLeak(mod, 'assimilate_cli_test');
assertStableToolingEnvelope({ status: 'ok', module: 'assimilate_cli' }, 'assimilate_cli_test');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
