#!/usr/bin/env node
'use strict';

const path = require('path');

const ADAPTIVE_IMPL = Object.freeze({
  layer_store: '../../core/layer1/memory_runtime/adaptive/layer_store.ts',
  habit_store: '../../core/layer1/memory_runtime/adaptive/habit_store.ts',
  reflex_store: '../../core/layer1/memory_runtime/adaptive/reflex_store.ts',
  strategy_store: '../../core/layer1/memory_runtime/adaptive/strategy_store.ts',
  catalog_store: '../../core/layer1/memory_runtime/adaptive/catalog_store.ts',
  focus_trigger_store: '../../core/layer1/memory_runtime/adaptive/focus_trigger_store.ts'
});

function loadAdaptiveMemoryModule(kind) {
  const key = String(kind || '').trim();
  const relPath = ADAPTIVE_IMPL[key];
  if (!relPath) {
    throw new Error(`adaptive_memory_bridge_unknown_kind:${key}`);
  }
  return require(path.resolve(__dirname, relPath));
}

exports.ADAPTIVE_IMPL = ADAPTIVE_IMPL;
exports.loadAdaptiveMemoryModule = loadAdaptiveMemoryModule;
module.exports = {
  ADAPTIVE_IMPL,
  loadAdaptiveMemoryModule
};
module.exports.default = module.exports;

export {};
