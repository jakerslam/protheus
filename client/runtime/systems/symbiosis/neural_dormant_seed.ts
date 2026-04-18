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

module.exports = bound;
