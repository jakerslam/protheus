#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/polyglot (thin interop bridge over semantic-kernel-bridge)

const bridge = require('../../client/runtime/systems/workflow/semantic_kernel_bridge.ts');

function registerBridge(payload = {}) {
  return bridge.registerDotnetBridge({
    bridge_path: 'adapters/polyglot/semantic_kernel_dotnet_bridge.ts',
    ...payload,
  });
}

function invokeBridge(payload = {}) {
  return bridge.invokeDotnetBridge(payload);
}

module.exports = {
  registerBridge,
  invokeBridge,
};
