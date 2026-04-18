#!/usr/bin/env node
'use strict';

const runtime = require('../../lib/legacy_retired_wrapper.ts');
const bound = runtime.bindLegacyRetiredModuleSafe(
  __filename,
  module,
  'full_virtual_desktop_claw_lane_target_missing_bind',
  'full_virtual_desktop_claw_lane_target_load_failed'
);

module.exports = bound;
