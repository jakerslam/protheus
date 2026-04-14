#!/usr/bin/env node
'use strict';

const bridge = require('../../client/runtime/systems/workflow/shannon_bridge.ts');
const BRIDGE_PATH = 'adapters/protocol/shannon_gateway_bridge.ts';
const FRAMEWORK = 'shannon';

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

function gatewayRoute(payload = {}) {
  return bridge.gatewayRoute(withBridgeMetadata(payload));
}

function registerTooling(payload = {}) {
  return bridge.registerTooling(withBridgeMetadata(payload));
}

function p2pReliability(payload = {}) {
  return bridge.p2pReliability(withBridgeMetadata(payload));
}

module.exports = {
  BRIDGE_PATH,
  FRAMEWORK,
  withBridgeMetadata,
  status,
  gatewayRoute,
  registerTooling,
  p2pReliability,
};
