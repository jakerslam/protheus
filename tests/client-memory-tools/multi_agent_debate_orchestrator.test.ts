#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Legacy JS test surface retired; authoritative checks are Rust-side.
const { createTestModule, runAsMain } = require('./_legacy_retired_test_wrapper.ts');
const mod = createTestModule(__dirname, 'multi_agent_debate_orchestrator.test', 'MEMORY-TEST-MULTI_AGENT_DEBATE_ORCHESTRATOR.TEST');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
