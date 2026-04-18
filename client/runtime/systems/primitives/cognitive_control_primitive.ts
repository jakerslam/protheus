#!/usr/bin/env node
'use strict';

const runtime = require('../../lib/legacy_retired_wrapper.ts');
const bound = runtime.bindLegacyRetiredModuleSafe(
  __filename,
  module,
  'cognitive_control_primitive_target_missing_bind',
  'cognitive_control_primitive_target_load_failed'
);

module.exports = bound;
