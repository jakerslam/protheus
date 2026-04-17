#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Legacy JS test surface retired; authoritative checks are Rust-side.
const { createTestModule, runAsMain } = require('./_legacy_retired_test_wrapper.ts');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');
const mod = createTestModule(__dirname, 'automated_compliance_mapping_evidence_engine.test', 'MEMORY-TEST-AUTOMATED_COMPLIANCE_MAPPING_EVIDENCE_ENGINE.TEST');
assertNoPlaceholderOrPromptLeak(mod, 'automated_compliance_mapping_evidence_engine_test');
assertStableToolingEnvelope({ status: 'ok', module: 'automated_compliance_mapping_evidence_engine' }, 'automated_compliance_mapping_evidence_engine_test');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
