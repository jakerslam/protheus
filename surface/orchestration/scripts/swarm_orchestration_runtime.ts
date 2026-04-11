#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Orchestration Surface coordination implementation (non-canonical).
// Layer ownership: surface/orchestration.

const { bindSwarmOrchestrationRuntimeModule } = require('../../../adapters/runtime/swarm_bridge_modules.ts');

module.exports = bindSwarmOrchestrationRuntimeModule(module);
