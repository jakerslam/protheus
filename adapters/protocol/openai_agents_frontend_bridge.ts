#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin OpenAI Agents frontend bridge over pydantic-ai-bridge)

const bridge = require('../../client/runtime/systems/workflow/pydantic_ai_bridge.ts');

function runGovernedWorkflow(payload = {}) {
  return bridge.runGovernedWorkflow({
    bridge_path: 'adapters/protocol/openai_agents_frontend_bridge.ts',
    framework: 'openai_agents',
    ...payload,
  });
}

module.exports = {
  runGovernedWorkflow,
};
