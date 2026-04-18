#!/usr/bin/env node
'use strict';

const runtime = require('../../lib/legacy_retired_wrapper.ts');
const bound = runtime.bindLegacyRetiredModuleSafe(
  __filename,
  module,
  'interactive_desktop_session_primitive_target_missing_bind',
  'interactive_desktop_session_primitive_target_load_failed'
);

module.exports = bound;
