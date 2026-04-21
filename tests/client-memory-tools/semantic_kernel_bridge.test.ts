#!/usr/bin/env node
'use strict';

// SRS coverage: V6-WORKFLOW-008.1, V6-WORKFLOW-008.2, V6-WORKFLOW-008.3,
// V6-WORKFLOW-008.4, V6-WORKFLOW-008.5, V6-WORKFLOW-008.6,
// V6-WORKFLOW-008.7, V6-WORKFLOW-008.8

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const ts = require('typescript');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

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

const bridge = require('../../client/runtime/systems/workflow/semantic_kernel_bridge.ts');

function run() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'semantic-kernel-bridge-'));
  const statePath = path.join(tmpDir, 'state.json');
  const historyPath = path.join(tmpDir, 'history.jsonl');
  const swarmStatePath = path.join(tmpDir, 'swarm-state.json');

  const service = bridge.registerService({
    name: 'semantic-kernel-enterprise',
    role: 'orchestrator',
    execution_surface: 'workflow-executor',
    default_budget: 720,
    capabilities: ['planning', 'plugins', 'memory'],
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(service.service.execution_surface, 'workflow-executor');

  const plugin = bridge.registerPlugin({
    service_id: service.service.service_id,
    plugin_name: 'faq_router',
    plugin_kind: 'prompt',
    bridge_path: 'adapters/cognition/skills/mcp/mcp_gateway.ts',
    entrypoint: 'invoke',
    prompt_template: 'Summarize {{topic}} for {{audience}}',
    state_path: statePath,
    history_path: historyPath,
  });
  const pluginResult = bridge.invokePlugin({
    plugin_id: plugin.plugin.plugin_id,
    args: { topic: 'semantic-kernel', audience: 'operators' },
    state_path: statePath,
    history_path: historyPath,
  });
  assert(String(pluginResult.invocation.rendered).includes('semantic-kernel'));

  const collaboration = bridge.collaborate({
    name: 'semantic-kernel-team',
    agents: [
      { label: 'planner', role: 'planner', task: 'plan enterprise request', budget: 240 },
      { label: 'executor', role: 'executor', task: 'execute enterprise request', budget: 240 },
    ],
    edges: [
      { from: 'planner', to: 'executor', relation: 'handoff', importance: 0.85, reason: 'planner_to_executor' },
    ],
    state_path: statePath,
    history_path: historyPath,
    swarm_state_path: swarmStatePath,
  });
  assert.strictEqual(Boolean(collaboration.collaboration.network_id), true);

  const plan = bridge.plan({
    service_id: service.service.service_id,
    objective: 'summarize and route an enterprise support case',
    functions: [
      { name: 'route', description: 'Route case to the right team', score: 0.7 },
      { name: 'summarize', description: 'Summarize the support case', score: 0.6 },
      { name: 'archive', description: 'Archive a completed case', score: 0.2 },
    ],
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(plan.plan.steps.length >= 2, true);

  const vector = bridge.registerVectorConnector({
    name: 'sk-memory',
    provider: 'memory-plane',
    context_budget_tokens: 80,
    documents: [
      { text: 'semantic kernel planner maps functions to workflow steps', metadata: { source: 'guide-1' } },
      { text: 'azure ai search connector is remote and rich-profile only', metadata: { source: 'guide-2' } },
    ],
    state_path: statePath,
    history_path: historyPath,
  });
  const retrieval = bridge.retrieve({
    connector_id: vector.connector.connector_id,
    query: 'planner workflow steps',
    profile: 'rich',
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(retrieval.results.length >= 1, true);

  const llm = bridge.registerLlmConnector({
    name: 'sk-azure-openai',
    provider: 'azure-openai',
    model: 'gpt-4.1',
    modalities: ['text', 'vision'],
    state_path: statePath,
    history_path: historyPath,
  });
  const route = bridge.routeLlm({
    connector_id: llm.connector.connector_id,
    modality: 'vision',
    profile: 'rich',
    prompt: 'Inspect the screenshot and classify the issue.',
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(route.route.modality, 'vision');

  const structured = bridge.validateStructuredOutput({
    schema: {
      type: 'object',
      required: ['answer'],
      properties: {
        answer: { type: 'string' },
        confidence: { type: 'number' },
      },
    },
    output: {
      answer: 'Use the workflow executor.',
      confidence: 0.91,
    },
    process: {
      steps: [
        { id: 'capture', next: 'route' },
        { id: 'route' },
      ],
    },
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(structured.record.process_report.validated, true);

  const enterprise = bridge.emitEnterpriseEvent({
    event_type: 'semantic-kernel.azure.deployment',
    sink: 'otel',
    cloud: 'azure',
    endpoint: 'https://example.azure.com/otel',
    deployment: { resource_group: 'rg-ops', service: 'aoai-prod' },
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(enterprise.event.cloud, 'azure');

  const status = bridge.status({ state_path: statePath, history_path: historyPath });
  assert.strictEqual(status.services, 1);
  assert.strictEqual(status.plugins, 1);
  assert.strictEqual(status.collaborations, 1);
  assert.strictEqual(status.plans, 1);
  assert.strictEqual(status.vector_connectors, 1);
  assert.strictEqual(status.llm_connectors, 1);
  assert.strictEqual(status.structured_processes, 1);
  assert.strictEqual(status.enterprise_events, 1);
  assertNoPlaceholderOrPromptLeak(status, 'semantic_kernel_bridge_test');
  assertStableToolingEnvelope(status, 'semantic_kernel_bridge_test');
  fs.rmSync(tmpDir, { recursive: true, force: true });
  console.log(JSON.stringify({ ok: true, type: 'semantic_kernel_bridge_test' }));
}

run();
