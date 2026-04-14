#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin OpenAI Agents frontend bridge over pydantic-ai-bridge)

const bridge = require('../../client/runtime/systems/workflow/pydantic_ai_bridge.ts');
const BRIDGE_PATH = 'adapters/protocol/openai_agents_frontend_bridge.ts';
const FRAMEWORK = 'openai_agents';

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
