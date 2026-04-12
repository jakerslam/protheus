#!/usr/bin/env node
'use strict';

// SRS coverage: V6-WORKFLOW-011.1, V6-WORKFLOW-011.2, V6-WORKFLOW-011.3,
// V6-WORKFLOW-011.4, V6-WORKFLOW-011.5, V6-WORKFLOW-011.6, V6-WORKFLOW-011.7,
// V6-WORKFLOW-011.8, V6-WORKFLOW-011.9

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

const bridge = require('../../client/runtime/systems/workflow/mastra_bridge.ts');
const runtimeBridge = require('../../adapters/polyglot/mastra_runtime_bridge.ts');
const mcpBridge = require('../../adapters/protocol/mastra_mcp_bridge.ts');

function run() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'mastra-bridge-'));
  const statePath = path.join(tmpDir, 'state.json');
  const historyPath = path.join(tmpDir, 'history.jsonl');
  const swarmStatePath = path.join(tmpDir, 'swarm-state.json');
  const approvalQueuePath = path.join(tmpDir, 'approvals.yaml');
  const outputDir = `client/runtime/local/state/mastra-shell-${process.pid}`;

  const runtime = runtimeBridge.registerBridge({
    name: 'mastra-python-gateway',
    language: 'python',
    provider: 'openai-compatible',
    model_family: 'gpt-5',
    models: ['gpt-5-mini'],
    supported_profiles: ['rich', 'pure'],
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(Boolean(runtime.runtime_bridge.bridge_id), true);

  const route = runtimeBridge.routeModel({
    bridge_id: runtime.runtime_bridge.bridge_id,
    language: 'python',
    provider: 'openai-compatible',
    model: 'gpt-5-mini',
    profile: 'pure',
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(route.route.reason_code, 'polyglot_runtime_requires_rich_profile');

  const graph = bridge.registerGraph({
    name: 'incident-graph',
    entrypoint: 'intake',
    nodes: [
      { id: 'intake', spawn: false },
      { id: 'research', parallel: true, budget: 128 },
      { id: 'draft', parallel: true, budget: 128 }
    ],
    edges: [
      { from: 'intake', to: 'research' },
      { from: 'intake', to: 'draft' }
    ],
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(Boolean(graph.graph.graph_id), true);

  const graphRun = bridge.executeGraph({
    graph_id: graph.graph.graph_id,
    profile: 'pure',
    state_path: statePath,
    history_path: historyPath,
    swarm_state_path: swarmStatePath
  });
  assert.strictEqual(graphRun.run.degraded, true);

  const mcp = mcpBridge.registerBridge({
    name: 'incident-resource-bridge',
    bridge_path: 'adapters/protocol/mastra_mcp_bridge.ts',
    supported_profiles: ['rich', 'pure'],
    requires_approval: false,
    capabilities: ['tools', 'resources'],
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(Boolean(mcp.mcp_bridge.tool_id), true);

  const agent = bridge.runAgentLoop({
    name: 'incident-agent',
    instruction: 'triage the incident and choose the right tool',
    runtime_bridge_id: runtime.runtime_bridge.bridge_id,
    language: 'python',
    provider: 'openai-compatible',
    model: 'gpt-5-mini',
    profile: 'rich',
    tools: [{ tool_id: mcp.mcp_bridge.tool_id, budget: 96 }],
    max_iterations: 2,
    state_path: statePath,
    history_path: historyPath,
    swarm_state_path: swarmStatePath
  });
  assert.deepStrictEqual(agent.agent.selected_tools, [mcp.mcp_bridge.tool_id]);

  const recall = bridge.memoryRecall({
    query: 'incident policy',
    top: 3,
    profile: 'pure',
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(Boolean(recall.memory_recall.ambient_receipt_path), true);

  const suspended = bridge.suspendRun({
    run_id: agent.agent.agent_id,
    summary: 'operator review needed',
    reason: 'escalate before execute',
    require_approval: true,
    state_path: statePath,
    history_path: historyPath,
    approval_queue_path: approvalQueuePath
  });
  assert.strictEqual(suspended.suspension.status, 'suspended');

  const approved = bridge.suspendRun({
    run_id: agent.agent.agent_id,
    action_id: suspended.suspension.approval.action_id,
    decision: 'approve',
    require_approval: true,
    state_path: statePath,
    history_path: historyPath,
    approval_queue_path: approvalQueuePath
  });
  assert.strictEqual(Boolean(approved.suspension.approval), true);

  const resumed = bridge.resumeRun({
    run_id: agent.agent.agent_id,
    state_path: statePath,
    history_path: historyPath,
    swarm_state_path: swarmStatePath,
    approval_queue_path: approvalQueuePath
  });
  assert.strictEqual(resumed.resume.status, 'resumed');

  const invocation = mcpBridge.invokeBridge({
    bridge_id: mcp.mcp_bridge.tool_id,
    profile: 'rich',
    args: { resource: 'incident-playbook' },
    state_path: statePath,
    history_path: historyPath,
    approval_queue_path: approvalQueuePath
  });
  assert.strictEqual(invocation.mcp_invocation.mode, 'mcp_tool_call');

  let capturedInvokePayload = null;
  const originalInvokeMcpBridge = bridge.invokeMcpBridge;
  bridge.invokeMcpBridge = (payload) => {
    capturedInvokePayload = payload;
    return { ok: true, intercepted: true };
  };
  try {
    const wrappedInvoke = mcpBridge.invokeBridge({ bridge_id: 'bridge-demo', args: { ping: true } });
    assert.strictEqual(wrappedInvoke.intercepted, true);
    assert.strictEqual(capturedInvokePayload.bridge_path, 'adapters/protocol/mastra_mcp_bridge.ts');
    assert.strictEqual(capturedInvokePayload.framework, 'mastra');
    assert.strictEqual(capturedInvokePayload.bridge_id, 'bridge-demo');
  } finally {
    bridge.invokeMcpBridge = originalInvokeMcpBridge;
  }

  const evaluation = bridge.recordEvalTrace({
    session_id: agent.agent.primary_session_id,
    profile: 'rich',
    score: 0.91,
    metrics: { tool_success: 1 },
    trace: [{ span: 'tool-call', ms: 22 }],
    token_telemetry: { prompt_tokens: 120, completion_tokens: 44 },
    log_summary: 'agent loop traced cleanly',
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(evaluation.evaluation.score, 0.91);
  assert.strictEqual(evaluation.evaluation.trace.length, 1);

  const deployment = bridge.deployShell({
    shell_name: 'mastra-studio',
    shell_path: 'client/runtime/systems/workflow/mastra_bridge.ts',
    target: 'local',
    artifact_path: 'apps/mastra-studio',
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(deployment.deployment.authority_delegate, 'core://mastra-bridge');

  const intake = bridge.scaffoldIntake({
    output_dir: outputDir,
    package_name: 'mastra-shell',
    state_path: statePath,
    history_path: historyPath
  });
  assert.strictEqual(
    fs.existsSync(path.join(process.cwd(), outputDir, 'package.json')),
    true
  );
  assert.strictEqual(Boolean(intake.intake.intake_id), true);

  const status = bridge.status({ state_path: statePath, history_path: historyPath });
  assert.strictEqual(status.graphs, 1);
  assert.strictEqual(status.graph_runs, 1);
  assert.strictEqual(status.agent_loops, 1);
  assert.strictEqual(status.memory_recalls, 1);
  assert.strictEqual(status.suspended_runs, 1);
  assert.strictEqual(status.mcp_bridges, 1);
  assert.strictEqual(status.eval_traces, 1);
  assert.strictEqual(status.deployments, 1);
  assert.strictEqual(status.runtime_bridges, 1);
  assert.strictEqual(status.intakes, 1);

  fs.rmSync(path.join(process.cwd(), outputDir), { recursive: true, force: true });

  console.log(JSON.stringify({ ok: true, type: 'mastra_bridge_test' }));
}

run();
