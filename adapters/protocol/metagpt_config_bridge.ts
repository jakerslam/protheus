#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin config bridge over metagpt-bridge)

const bridge = require('../../client/runtime/systems/workflow/metagpt_bridge.ts');
const BRIDGE_PATH = 'adapters/protocol/metagpt_config_bridge.ts';
const FRAMEWORK = 'metagpt';

function withBridgeMetadata(payload = {}) {
  return {
    bridge_path: BRIDGE_PATH,
    framework: FRAMEWORK,
    ...payload,
  };
}

function status(payload = {}) {
  return bridge.status(withBridgeMetadata(payload));
}

function simulatePr(payload = {}) {
  return bridge.simulatePr(withBridgeMetadata(payload));
}

function ingestConfig(payload = {}) {
  return bridge.ingestConfig(withBridgeMetadata(payload));
}

module.exports = {
  BRIDGE_PATH,
  FRAMEWORK,
  withBridgeMetadata,
  status,
  simulatePr,
  ingestConfig,
};
