#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Legacy JS test surface retired; authoritative checks are Rust-side.
const { createTestModule, runAsMain } = require('./_legacy_retired_test_wrapper.ts');
const mod = createTestModule(__dirname, 'protheusctl_migrate_openfang_alias.test', 'MEMORY-TEST-PROTHEUSCTL_MIGRATE_OPENFANG_ALIAS.TEST');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
