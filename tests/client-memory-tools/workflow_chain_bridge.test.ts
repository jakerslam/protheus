#!/usr/bin/env node
'use strict';

// SRS coverage: V6-WORKFLOW-014.1, V6-WORKFLOW-014.2, V6-WORKFLOW-014.3,
// V6-WORKFLOW-014.4, V6-WORKFLOW-014.5, V6-WORKFLOW-014.6, V6-WORKFLOW-014.7,
// V6-WORKFLOW-014.8, V6-WORKFLOW-014.9

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
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

const bridge = require('../../client/runtime/systems/workflow/workflow_chain_bridge.ts');
const connectorBridge = require('../../adapters/protocol/workflow_chain_connector_bridge.ts');

function run() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'workflow_chain-bridge-'));
  const statePath = path.join(tmpDir, 'state.json');
  const historyPath = path.join(tmpDir, 'history.jsonl');
  const swarmStatePath = path.join(tmpDir, 'swarm-state.json');
  const outputDir = `client/runtime/local/state/workflow_chain-shell-${process.pid}`;

  const chain = bridge.registerChain({
    name: 'incident-chain',
    runnables: [
      { id: 'retrieve', runnable_type: 'retriever', parallel: true, budget: 192 },
      { id: 'rank', runnable_type: 'ranker', parallel: true, budget: 160 },
      { id: 'answer', runnable_type: 'llm', spawn: true, budget: 256 }
    ],
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(Boolean(chain.chain.chain_id), true);

  const middleware = bridge.registerMiddleware({
    name: 'incident-before-chain',
    chain_id: chain.chain.chain_id,
    hook: 'before_chain',
    action: 'attach_replay_metadata',
    fail_closed: true,
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(Boolean(middleware.middleware.middleware_id), true);

  const chainRun = bridge.executeChain({
    chain_id: chain.chain.chain_id,
    profile: 'pure',
    state_path: statePath,
    history_path: historyPath,
    swarm_state_path: swarmStatePath
  });
  assert.strictEqual(chainRun.run.degraded, true);
  assert.strictEqual(chainRun.run.middleware_count, 1);

  const agent = bridge.runDeepAgent({
    name: 'incident-deep-agent',
    instruction: 'triage billing incident and choose tools',
    profile: 'pure',
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

  const memory = connectorBridge.registerMemoryBridge({
    name: 'incident-memory',
    documents: [
      { text: 'billing incident playbook', metadata: { kind: 'graph', source: 'playbook' } },
      { text: 'general faq on accounts', metadata: { kind: 'faq', source: 'faq' } }
    ],
    state_path: statePath,
    history_path: historyPath
  });
  const recall = connectorBridge.recallMemory({
    memory_id: memory.memory_bridge.memory_id,
    query: 'billing incident',
    mode: 'hybrid',
    profile: 'pure',
    top_k: 4,
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(recall.recall.degraded, true);
  assert.strictEqual(recall.recall.results.length >= 1, true);

  const integration = connectorBridge.importIntegration({
    name: 'workflow_chain-qdrant',
    integration_type: 'vector-store',
    assets: [{ kind: 'package', name: '@workflow_chain/qdrant' }],
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(Boolean(integration.integration.integration_id), true);

  const prompt = bridge.routePrompt({
    name: 'incident-prompt',
    profile: 'pure',
    provider: 'frontier_provider',
    fallback_provider: 'openai-compatible',
    model: 'claude-3-7-sonnet',
    template: 'Answer {{question}} with {{context}}',
    variables: { question: 'What happened?', context: 'billing service degraded' },
    supported_providers: ['frontier_provider', 'openai-compatible'],
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(prompt.route.degraded, true);
  assert.strictEqual(prompt.route.selected_provider, 'openai-compatible');
  assert.strictEqual(prompt.route.rendered_prompt, 'Answer What happened? with billing service degraded');

  const parsed = bridge.parseStructuredOutput({
    name: 'incident-json',
    schema: {
      required_fields: ['answer', 'confidence'],
      field_types: { answer: 'string', confidence: 'number' }
    },
    output_json: { answer: 'Billing service degraded', confidence: 0.91 },
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(parsed.structured_output.validated_output.answer, 'Billing service degraded');

  const trace = bridge.recordTrace({
    trace_id: 'incident-trace',
    steps: [
      { stage: 'retrieve', message: 'retrieved evidence' },
      { stage: 'answer', message: 'drafted response' }
    ],
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(trace.trace.steps.length, 2);

  const checkpoint = bridge.checkpointRun({
    chain_id: chain.chain.chain_id,
    profile: 'pure',
    prototype_label: 'incident-fast-loop',
    state_snapshot: { retrieved: 2, drafted: true },
    state_path: statePath,
    history_path: historyPath,
    swarm_state_path: swarmStatePath
  });
  assert.strictEqual(Boolean(checkpoint.checkpoint.checkpoint_id), true);
  assert.strictEqual(checkpoint.checkpoint.degraded, true);

  const intake = bridge.assimilateIntake({
    output_dir: outputDir,
    package_name: 'workflow_chain-shell',
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(fs.existsSync(path.join(process.cwd(), outputDir, 'package.json')), true);
  assert.strictEqual(Boolean(intake.intake.intake_id), true);

  const status = bridge.status({ state_path: statePath, history_path: historyPath });
  assert.strictEqual(status.chains, 1);
  assert.strictEqual(status.chain_runs, 1);
  assert.strictEqual(status.middleware_hooks, 1);
  assert.strictEqual(status.agent_runs, 1);
  assert.strictEqual(status.memory_bridges, 1);
  assert.strictEqual(status.memory_queries, 1);
  assert.strictEqual(status.integrations, 1);
  assert.strictEqual(status.prompt_routes, 1);
  assert.strictEqual(status.structured_outputs, 1);
  assert.strictEqual(status.traces, 1);
  assert.strictEqual(status.checkpoints, 1);
  assert.strictEqual(status.intakes, 1);

  fs.rmSync(path.join(process.cwd(), outputDir), { recursive: true, force: true });

  console.log(JSON.stringify({ ok: true, type: 'workflow_chain_bridge_test' }));
}

run();
