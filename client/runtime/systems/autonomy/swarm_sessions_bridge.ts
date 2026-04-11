#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops (authoritative transport + receipts); this file is a thin bridge wrapper.

const { bindSwarmSessionsBridgeModule } = require('../../../../adapters/runtime/swarm_bridge_modules.ts');

module.exports = bindSwarmSessionsBridgeModule(module);
