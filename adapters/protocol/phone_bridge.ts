#!/usr/bin/env node
'use strict';

// thin desktop shell only

const bridge = require('../../client/runtime/lib/phone_runtime_bridge.ts');
const BRIDGE_PATH = 'adapters/protocol/phone_bridge.ts';
const FRAMEWORK = 'phone_runtime';

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

function sensorIntake(payload = {}) {
  return bridge.sensorIntake(withBridgeMetadata(payload));
}

function interactionMode(payload = {}) {
  return bridge.interactionMode(withBridgeMetadata(payload));
}

function backgroundDaemon(payload = {}) {
  return bridge.backgroundDaemon(withBridgeMetadata(payload));
}

module.exports = {
  BRIDGE_PATH,
  FRAMEWORK,
  withBridgeMetadata,
  status,
  sensorIntake,
  interactionMode,
  backgroundDaemon,
};
