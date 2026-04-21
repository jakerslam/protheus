#!/usr/bin/env node
'use strict';

const runtime = require('../../lib/legacy_retired_wrapper.ts');
const bound = runtime.loadBoundModuleFromRuntime(
  runtime,
  __filename,
  module,
  'neural_dormant_seed_target_missing_bind',
  'neural_dormant_seed_target_load_failed'
);
runtime.exitIfBoundModuleFailed(bound, module);

function assertBoundContract(value) {
  if (!value || (typeof value !== 'function' && typeof value !== 'object')) {
    throw new Error('neural_dormant_seed_invalid_bound_contract');
  }
  return value;
}

const BINDING_METADATA = Object.freeze({
  ok: true,
  type: 'neural_dormant_seed_binding',
  source: 'legacy_retired_wrapper',
  lane: 'symbiosis',
  contract_version: '2026-04-20'
});

const verified = assertBoundContract(bound);
if (verified && (typeof verified === 'object' || typeof verified === 'function')) {
  Object.defineProperty(verified, '__bridge_meta', {
    configurable: false,
    enumerable: true,
    writable: false,
    value: BINDING_METADATA
  });
}

module.exports = verified;
