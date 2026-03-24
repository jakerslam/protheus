#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Legacy JS test surface retired; authoritative checks are Rust-side.
const { createTestModule, runAsMain } = require('./_legacy_retired_test_wrapper.ts');
const mod = createTestModule(__dirname, 'v4_mesh_runtime_pack.test', 'MEMORY-TEST-V4_MESH_RUNTIME_PACK.TEST');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
