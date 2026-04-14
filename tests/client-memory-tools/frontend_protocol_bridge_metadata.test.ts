#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const ts = require('typescript');

if (!require.extensions['.ts']) {
  require.extensions['.ts'] = function compileTs(module, filename) {
    const source = fs.readFileSync(filename, 'utf8');
    const transpiled = ts.transpileModule(source, {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2022,
        moduleResolution: ts.ModuleResolutionKind.NodeJs,
        esModuleInterop: true,
        allowSyntheticDefaultImports: true
      },
      fileName: filename,
      reportDiagnostics: false
    }).outputText;
    module._compile(transpiled, filename);
  };
}

function run() {
  const workflowGraphBridge = require('../../client/runtime/systems/workflow/workflow_graph_bridge.ts');
  const pydanticBridge = require('../../client/runtime/systems/workflow/pydantic_ai_bridge.ts');

  const originalGraphStatus = workflowGraphBridge.status;
  const originalGraphRun = workflowGraphBridge.runGovernedWorkflow;
  const originalPydanticStatus = pydanticBridge.status;
  const originalPydanticRun = pydanticBridge.runGovernedWorkflow;

  workflowGraphBridge.status = (payload = {}) => payload;
  workflowGraphBridge.runGovernedWorkflow = (payload = {}) => payload;
  pydanticBridge.status = (payload = {}) => payload;
  pydanticBridge.runGovernedWorkflow = (payload = {}) => payload;

  try {
    const langgraph = require('../../adapters/protocol/langgraph_frontend_bridge.ts');
    const openaiAgents = require('../../adapters/protocol/openai_agents_frontend_bridge.ts');

    const langStatus = langgraph.status({ trace: 'lang-status' });
    assert.equal(langStatus.bridge_path, 'adapters/protocol/langgraph_frontend_bridge.ts');
    assert.equal(langStatus.framework, 'langgraph');
    assert.equal(langStatus.trace, 'lang-status');

    const langRun = langgraph.runGovernedWorkflow({ objective: 'graph-plan' });
    assert.equal(langRun.bridge_path, 'adapters/protocol/langgraph_frontend_bridge.ts');
    assert.equal(langRun.framework, 'langgraph');
    assert.equal(langRun.objective, 'graph-plan');

    const openaiStatus = openaiAgents.status({ trace: 'oa-status' });
    assert.equal(openaiStatus.bridge_path, 'adapters/protocol/openai_agents_frontend_bridge.ts');
    assert.equal(openaiStatus.framework, 'openai_agents');
    assert.equal(openaiStatus.trace, 'oa-status');

    const openaiRun = openaiAgents.runGovernedWorkflow({ objective: 'agent-plan' });
    assert.equal(openaiRun.bridge_path, 'adapters/protocol/openai_agents_frontend_bridge.ts');
    assert.equal(openaiRun.framework, 'openai_agents');
    assert.equal(openaiRun.objective, 'agent-plan');
  } finally {
    workflowGraphBridge.status = originalGraphStatus;
    workflowGraphBridge.runGovernedWorkflow = originalGraphRun;
    pydanticBridge.status = originalPydanticStatus;
    pydanticBridge.runGovernedWorkflow = originalPydanticRun;
  }

  console.log(JSON.stringify({ ok: true, type: 'frontend_protocol_bridge_metadata_test' }));
}

run();
