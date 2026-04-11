#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: surface/orchestration (swarm orchestration coordination); this file is a thin CLI bridge.

const { bindSwarmSurfaceModule } = require('../../../../adapters/runtime/swarm_bridge_modules.ts');

module.exports = bindSwarmSurfaceModule('swarm_orchestration_runtime', module);
