#!/usr/bin/env node
'use strict';
// Orchestration Surface coordination implementation (non-canonical).
// Layer ownership: surface/orchestration.

const { bindRuntimeSystemModule } = require('../../../adapters/runtime/runtime_system_bridge.ts');

module.exports = bindRuntimeSystemModule(
  __dirname,
  'self_improving_redteam_trainer',
  'SYSTEMS-REDTEAM-SELF_IMPROVING_REDTEAM_TRAINER',
  module,
);
