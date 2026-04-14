#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin protocol bridge over crewai-bridge)

const bridge = require('../../client/runtime/systems/workflow/crewai_bridge.ts');
const BRIDGE_PATH = 'adapters/protocol/crewai_tool_bridge.ts';
const FRAMEWORK = 'crewai';

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

function registerCrew(payload = {}) {
  return bridge.registerCrew(withBridgeMetadata(payload));
}

function runProcess(payload = {}) {
  return bridge.runProcess(withBridgeMetadata(payload));
}

function runFlow(payload = {}) {
  return bridge.runFlow(withBridgeMetadata(payload));
}

function memoryBridge(payload = {}) {
  return bridge.memoryBridge(withBridgeMetadata(payload));
}

function ingestConfig(payload = {}) {
  return bridge.ingestConfig(withBridgeMetadata(payload));
}

function routeDelegation(payload = {}) {
  return bridge.routeDelegation(withBridgeMetadata(payload));
}

function routeModel(payload = {}) {
  return bridge.routeModel(withBridgeMetadata(payload));
}

function reviewCrew(payload = {}) {
  return bridge.reviewCrew(withBridgeMetadata(payload));
}

function recordAmpTrace(payload = {}) {
  return bridge.recordAmpTrace(withBridgeMetadata(payload));
}

function benchmarkParity(payload = {}) {
  return bridge.benchmarkParity(withBridgeMetadata(payload));
}

function runGovernedWorkflow(payload = {}) {
  return bridge.runGovernedWorkflow(withBridgeMetadata(payload));
}

module.exports = {
  BRIDGE_PATH,
  FRAMEWORK,
  withBridgeMetadata,
  status,
  registerCrew,
  runProcess,
  runFlow,
  memoryBridge,
  ingestConfig,
  routeDelegation,
  routeModel,
  reviewCrew,
  recordAmpTrace,
  benchmarkParity,
  runGovernedWorkflow,
};
