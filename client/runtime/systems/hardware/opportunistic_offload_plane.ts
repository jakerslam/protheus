#!/usr/bin/env node
'use strict';

const runtime = require('../../lib/legacy_retired_wrapper.ts');
const bound = runtime.bindLegacyRetiredModuleSafe(
  __filename,
  module,
  'opportunistic_offload_plane_target_missing_bind',
  'opportunistic_offload_plane_target_load_failed'
);

module.exports = bound;
