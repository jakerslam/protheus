#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: client/runtime/systems/workflow (thin bridge over client/runtime/lib).

const impl = require('../../lib/dify_bridge.ts');
const BRIDGE_PATH = 'client/runtime/systems/workflow/dify_bridge.ts';
const BRIDGE_TARGET = 'client/runtime/lib/dify_bridge.ts';
const FRAMEWORK = 'dify';

function withBridgeMetadata(payload = {}) {
  return {
    bridge_path: BRIDGE_PATH,
    framework: FRAMEWORK,
    ...payload,
  };
}

module.exports = {
  BRIDGE_PATH,
  BRIDGE_TARGET,
  FRAMEWORK,
  withBridgeMetadata,
  ...impl,
};
