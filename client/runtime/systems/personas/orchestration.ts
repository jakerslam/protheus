#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: surface/orchestration (persona orchestration coordination); this file is a thin CLI bridge.

const { bindOrchestrationSurfaceModule } = require('../../../../adapters/runtime/orchestration_surface_modules.ts');

module.exports = bindOrchestrationSurfaceModule('personas_orchestration', module);
