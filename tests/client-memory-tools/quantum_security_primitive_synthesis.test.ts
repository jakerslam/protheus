#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Legacy JS test surface retired; authoritative checks are Rust-side.
const { bindLegacyRetiredTest } = require('./_legacy_retired_test_wrapper.ts');
module.exports = bindLegacyRetiredTest(
  module,
  __dirname,
  'quantum_security_primitive_synthesis.test',
  'MEMORY-TEST-QUANTUM_SECURITY_PRIMITIVE_SYNTHESIS.TEST'
);
