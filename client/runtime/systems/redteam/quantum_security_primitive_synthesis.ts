#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: surface/orchestration (redteam coordination); this file is a thin CLI bridge.
const { bindCompatibilityBridgeModule } = require('../../lib/legacy_retired_wrapper.ts');

module.exports = bindCompatibilityBridgeModule(
  require.resolve('../../../../surface/orchestration/scripts/quantum_security_primitive_synthesis.ts'),
  module
);
