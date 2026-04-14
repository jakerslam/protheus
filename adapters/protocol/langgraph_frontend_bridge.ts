#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin LangGraph frontend bridge over workflow_graph-bridge)

const bridge = require('../../client/runtime/systems/workflow/workflow_graph_bridge.ts');
const BRIDGE_PATH = 'adapters/protocol/langgraph_frontend_bridge.ts';
const FRAMEWORK = 'langgraph';

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

function runGovernedWorkflow(payload = {}) {
  return bridge.runGovernedWorkflow(withBridgeMetadata(payload));
}

module.exports = {
  BRIDGE_PATH,
  FRAMEWORK,
  withBridgeMetadata,
  status,
  runGovernedWorkflow,
};
