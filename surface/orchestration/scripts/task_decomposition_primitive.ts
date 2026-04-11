#!/usr/bin/env node
'use strict';
// Orchestration Surface coordination implementation (non-canonical).
// Layer ownership: surface/orchestration.

const { bindRuntimeSystemModule } = require('../../../adapters/runtime/runtime_system_bridge.ts');

module.exports = bindRuntimeSystemModule(
  __dirname,
  'task_decomposition_primitive',
  'SYSTEMS-EXECUTION-TASK_DECOMPOSITION_PRIMITIVE',
  module,
);
