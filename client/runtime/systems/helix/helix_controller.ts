#!/usr/bin/env node
'use strict';

const runtime = require('../../lib/legacy_retired_wrapper.ts');
const bound = runtime.bindLegacyRetiredModuleSafe(
  __filename,
  module,
  'helix_controller_target_missing_bind',
  'helix_controller_target_load_failed'
);

module.exports = bound;
