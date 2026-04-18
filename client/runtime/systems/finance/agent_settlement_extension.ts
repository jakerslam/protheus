#!/usr/bin/env node
'use strict';

const runtime = require('../../lib/legacy_retired_wrapper.ts');
const bound = runtime.bindLegacyRetiredModuleSafe(
  __filename,
  module,
  'agent_settlement_extension_target_missing_bind',
  'agent_settlement_extension_target_load_failed'
);

module.exports = bound;
