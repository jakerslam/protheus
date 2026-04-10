#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Legacy JS test surface retired; authoritative checks are Rust-side.
const { bindLegacyRetiredTest } = require('./_legacy_retired_test_wrapper.ts');
module.exports = bindLegacyRetiredTest(
  module,
  __dirname,
  'autonomy_strategy_trit_shadow_ranking_summary_rust_parity.test',
  'MEMORY-TEST-AUTONOMY_STRATEGY_TRIT_SHADOW_RANKING_SUMMARY_RUST_PARITY.TEST'
);
