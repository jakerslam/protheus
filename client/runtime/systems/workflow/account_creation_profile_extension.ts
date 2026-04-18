#!/usr/bin/env node
'use strict';

const runtime = require('../../lib/legacy_retired_wrapper.ts');
const bound = runtime.bindLegacyRetiredModuleSafe(
  __filename,
  module,
  'account_creation_profile_extension_target_missing_bind',
  'account_creation_profile_extension_target_load_failed'
);

module.exports = bound;
