#!/usr/bin/env node
'use strict';

// SRS coverage: V6-WORKFLOW-012.1, V6-WORKFLOW-012.2, V6-WORKFLOW-012.3,
// V6-WORKFLOW-012.4, V6-WORKFLOW-012.5, V6-WORKFLOW-012.6, V6-WORKFLOW-012.7,
// V6-WORKFLOW-012.8

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const ts = require('typescript');\nconst { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

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

const bridge = require('../../client/runtime/systems/workflow/haystack_bridge.ts');
const connectorBridge = require('../../adapters/protocol/haystack_connector_bridge.ts');

function run() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'haystack-bridge-'));
  const statePath = path.join(tmpDir, 'state.json');
  const historyPath = path.join(tmpDir, 'history.jsonl');
  const swarmStatePath = path.join(tmpDir, 'swarm-state.json');
  const outputDir = `client/runtime/local/state/haystack-shell-${process.pid}`;

  const pipeline = bridge.registerPipeline({
    name: 'incident-pipeline',
    components: [
      { id: 'retrieve', stage_type: 'retriever', parallel: true, budget: 192 },
      { id: 'rank', stage_type: 'ranker', parallel: true, budget: 160 },
      { id: 'answer', stage_type: 'generator', spawn: true, budget: 256 }
    ],
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(Boolean(pipeline.pipeline.pipeline_id), true);

  const pipelineRun = bridge.runPipeline({
    pipeline_id: pipeline.pipeline.pipeline_id,
    profile: 'pure',
    state_path: statePath,
    history_path: historyPath,
    swarm_state_path: swarmStatePath
  });
  assert.strictEqual(pipelineRun.run.degraded, true);

  const agent = bridge.runAgentToolset({
    name: 'incident-agent',
    goal: 'triage billing incident',
    search_limit: 2,
    tools: [
      { name: 'billing_lookup', description: 'billing incident ledger lookup', tags: ['billing', 'incident'] },
      { name: 'general_faq', description: 'general frequently asked questions', tags: ['faq'] },
      { name: 'ops_console', description: 'operational incident tool', tags: ['incident'] }
    ],
    state_path: statePath,
    history_path: historyPath,
    swarm_state_path: swarmStatePath
  });
  assert.strictEqual(agent.agent.selected_tools.length >= 1, true);

  const template = bridge.registerTemplate({
    name: 'incident-template',
    template: 'Answer {{question}} with {{context}}',
    state_path: statePath,
    history_path: historyPath
  });
  const rendered = bridge.renderTemplate({
    template_id: template.template.template_id,
    variables: { question: 'What happened?', context: 'billing service degraded' },
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(rendered.render.output, 'Answer What happened? with billing service degraded');

  const store = connectorBridge.registerDocumentStore({
    name: 'incident-docs',
    documents: [
      { text: 'billing incident playbook', metadata: { kind: 'graph', source: 'playbook' } },
      { text: 'general faq on accounts', metadata: { kind: 'faq', source: 'faq' } }
    ],
    state_path: statePath,
    history_path: historyPath
  });
  const retrieval = connectorBridge.retrieveDocuments({
    store_id: store.document_store.store_id,
    query: 'billing incident',
    mode: 'hybrid',
    profile: 'pure',
    top_k: 4,
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(retrieval.retrieval.degraded, true);
  assert.strictEqual(retrieval.retrieval.results.length >= 1, true);

  const route = bridge.routeAndRank({
    name: 'incident-router',
    query: 'billing escalation',
    context: { intent: 'billing' },
    routes: [
      { id: 'billing', field: 'intent', equals: 'billing', reason: 'billing route' },
      { id: 'general', field: 'intent', equals: 'general', reason: 'general route' }
    ],
    candidates: [
      { text: 'billing policy doc', metadata: { kind: 'policy' } },
      { text: 'generic faq', metadata: { kind: 'faq' } }
    ],
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(route.route.selected_route.id, 'billing');

  const evalRow = bridge.recordMultimodalEval({
    name: 'incident-eval',
    profile: 'pure',
    artifacts: [
      { media_type: 'image/png', path: 'adapters/assets/incident.png' },
      { media_type: 'text/plain', path: 'adapters/assets/incident.txt' }
    ],
    metrics: { faithfulness: 0.93 },
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(evalRow.evaluation.degraded, true);

  const trace = bridge.traceRun({
    trace_id: 'incident-trace',
    steps: [
      { stage: 'retrieve', message: 'retrieved evidence' },
      { stage: 'answer', message: 'drafted response' }
    ],
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(trace.trace.steps.length, 2);

  const connector = connectorBridge.registerConnector({
    name: 'haystack-qdrant',
    connector_type: 'qdrant',
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(Boolean(connector.connector.connector_id), true);

  const intake = bridge.assimilateIntake({
    output_dir: outputDir,
    package_name: 'haystack-shell',
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(fs.existsSync(path.join(process.cwd(), outputDir, 'package.json')), true);
  assert.strictEqual(Boolean(intake.intake.intake_id), true);

  const status = bridge.status({ state_path: statePath, history_path: historyPath });
  assert.strictEqual(status.pipelines, 1);
  assert.strictEqual(status.pipeline_runs, 1);
  assert.strictEqual(status.agent_runs, 1);
  assert.strictEqual(status.templates, 1);
  assert.strictEqual(status.template_renders, 1);
  assert.strictEqual(status.document_stores, 1);
  assert.strictEqual(status.retrieval_runs, 1);
  assert.strictEqual(status.routes, 1);
  assert.strictEqual(status.evaluations, 1);
  assert.strictEqual(status.traces, 1);
  assert.strictEqual(status.connectors, 1);
  assert.strictEqual(status.intakes, 1);

  fs.rmSync(path.join(process.cwd(), outputDir), { recursive: true, force: true });\n  assertNoPlaceholderOrPromptLeak(status, 'haystack_bridge_test');\n  assertStableToolingEnvelope(status, 'haystack_bridge_test');\n  console.log(JSON.stringify({ ok: true, type: 'haystack_bridge_test' }));
}

run();
