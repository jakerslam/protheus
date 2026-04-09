#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/polyglot (thin runtime bridge over mastra-bridge)

const bridge = require('../../client/runtime/systems/workflow/mastra_bridge.ts');

function registerBridge(payload = {}) {
  return bridge.registerRuntimeBridge({
    bridge_path: 'adapters/polyglot/mastra_runtime_bridge.ts',
    ...payload,
  });
}

function routeModel(payload = {}) {
  return bridge.routeModel(payload);
}

module.exports = {
  registerBridge,
  routeModel,
};
