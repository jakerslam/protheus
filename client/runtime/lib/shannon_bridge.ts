#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::shannon-bridge (authoritative shared workflow bridge).

const impl = require('../../../adapters/runtime/shannon_bridge.ts');
const BRIDGE_PATH = 'client/runtime/lib/shannon_bridge.ts';
const BRIDGE_TARGET = 'adapters/runtime/shannon_bridge.ts';
const FRAMEWORK = 'shannon';

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
