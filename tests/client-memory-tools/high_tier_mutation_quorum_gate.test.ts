#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Legacy JS test surface retired; authoritative checks are Rust-side.
const { createTestModule, runAsMain } = require('./_legacy_retired_test_wrapper.ts');
const mod = createTestModule(__dirname, 'high_tier_mutation_quorum_gate.test', 'MEMORY-TEST-HIGH_TIER_MUTATION_QUORUM_GATE.TEST');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
