#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin protocol bridge over dspy-bridge)

const bridge = require('../../client/runtime/systems/workflow/dspy_bridge.ts');
const BRIDGE_PATH = 'adapters/protocol/dspy_program_bridge.ts';
const FRAMEWORK = 'dspy';

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

function importIntegration(payload = {}) {
  return bridge.importIntegration(withBridgeMetadata(payload));
}

function executeMultihop(payload = {}) {
  return bridge.executeMultihop(withBridgeMetadata(payload));
}

function recordBenchmark(payload = {}) {
  return bridge.recordBenchmark(withBridgeMetadata(payload));
}

module.exports = {
  BRIDGE_PATH,
  FRAMEWORK,
  withBridgeMetadata,
  status,
  importIntegration,
  executeMultihop,
  recordBenchmark,
};
