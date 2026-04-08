#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin LangGraph frontend bridge over workflow_graph-bridge)

const bridge = require('../../client/runtime/systems/workflow/workflow_graph_bridge.ts');

function runGovernedWorkflow(payload = {}) {
  return bridge.runGovernedWorkflow({
    bridge_path: 'adapters/protocol/langgraph_frontend_bridge.ts',
    framework: 'langgraph',
    ...payload,
  });
}

module.exports = {
  runGovernedWorkflow,
};
