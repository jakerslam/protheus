#!/usr/bin/env node
'use strict';
// Orchestration Surface coordination implementation (non-canonical).
// Layer ownership: surface/orchestration.

const { bindRuntimeSystemModule } = require('../../../adapters/runtime/runtime_system_bridge.ts');

module.exports = bindRuntimeSystemModule(
  __dirname,
  'quantum_security_primitive_synthesis',
  'SYSTEMS-REDTEAM-QUANTUM_SECURITY_PRIMITIVE_SYNTHESIS',
  module,
);
