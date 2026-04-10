#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime (bridge) -> core/layer1/memory_runtime/adaptive (authoritative)

const { loadAdaptiveMemoryModule } = require('../../../../../../adapters/runtime/adaptive_memory_bridge.ts');

module.exports = loadAdaptiveMemoryModule('catalog_store');

export {};
