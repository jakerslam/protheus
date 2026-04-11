#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: surface/orchestration (research coordination); this file is a thin CLI bridge.
module.exports = require('../../../../adapters/runtime/orchestration_surface_modules.ts').bindOrchestrationSurfaceModule('research_organ', module);
