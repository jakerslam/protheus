#!/usr/bin/env node
'use strict';

// SRS coverage: V6-WORKFLOW-007.1, V6-WORKFLOW-007.2, V6-WORKFLOW-007.3,
// V6-WORKFLOW-007.4, V6-WORKFLOW-007.5, V6-WORKFLOW-007.7

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');

const bridge = require('../../client/runtime/systems/autonomy/swarm_sessions_bridge.ts');

function run() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'swarm-workflow-007-'));
  const state = path.join(tmpDir, 'state.json');

  const coordinator = bridge.sessionsSpawn({
    task: 'workflow-007 coordinator',
    max_tokens: 1024,
    on_budget_exhausted: 'fail',
    agentLabel: 'workflow-007-coordinator',
    state_path: state,
  });
  const specialist = bridge.sessionsSpawn({
    task: 'workflow-007 specialist',
    session_id: coordinator.session_id,
    max_tokens: 192,
    on_budget_exhausted: 'fail',
    agentLabel: 'workflow-007-specialist',
    state_path: state,
  });

  const contextPut = bridge.sessionsContextPut({
    session_id: coordinator.session_id,
    context: {
      objective: 'workflow-007',
      note: 'delegate governed work',
      oversized: 'x'.repeat(9000),
    },
    state_path: state,
  });
  assert.strictEqual(
    contextPut.payload.receipt.degraded_mode,
    'context_compacted',
    'expected oversized context to compact under budget pressure'
  );

  const handoff = bridge.sessionsHandoff({
    session_id: coordinator.session_id,
    target_session_id: specialist.session_id,
    reason: 'delegate specialist analysis',
    importance: 0.85,
    context: {
      delegated_goal: 'produce governed answer',
      owner: 'workflow-007-coordinator',
    },
    state_path: state,
  });
  assert.strictEqual(Boolean(handoff.payload.handoff.handoff_id), true);
  assert.strictEqual(handoff.payload.handoff.sender_session_id, coordinator.session_id);
  assert.strictEqual(handoff.payload.handoff.recipient_session_id, specialist.session_id);

  const specialistContext = bridge.sessionsContextGet({
    session_id: specialist.session_id,
    state_path: state,
  });
  assert.strictEqual(specialistContext.context.delegated_goal, 'produce governed answer');

  const specialistState = bridge.sessionsState({
    session_id: specialist.session_id,
    state_path: state,
  });
  assert(
    Array.isArray(specialistState.payload.handoffs) && specialistState.payload.handoffs.length >= 1,
    'expected session state to expose handoff registry rows'
  );

  const toolManifest = bridge.toolsRegisterJsonSchema({
    session_id: specialist.session_id,
    toolName: 'context_patch',
    schema: {
      type: 'object',
      properties: {
        context: { type: 'object' },
        merge: { type: 'boolean' },
      },
      required: ['context'],
    },
    bridgePath: 'client/runtime/systems/autonomy/swarm_sessions_bridge.ts',
    entrypoint: 'sessions_context_put',
    description: 'Patch governed context via the swarm bridge.',
    state_path: state,
  });
  assert.strictEqual(toolManifest.payload.tool_manifest.policy.fail_closed, true);

  const toolResult = bridge.toolsInvoke({
    session_id: specialist.session_id,
    toolName: 'context_patch',
    args: {
      context: {
        tool_applied: true,
        tool_source: 'workflow-007-bridge-test',
      },
      merge: true,
    },
    state_path: state,
  });
  assert.strictEqual(toolResult.payload.result.receipt.context.tool_applied, true);

  let unsafeDenied = false;
  try {
    bridge.toolsRegisterJsonSchema({
      session_id: specialist.session_id,
      toolName: 'unsafe_tool',
      schema: { type: 'object' },
      bridgePath: '../unsafe/bridge.ts',
      entrypoint: 'noop',
      state_path: state,
    });
  } catch (error) {
    unsafeDenied = /unsafe_tool_bridge|unsupported_tool_bridge|tools_register_json_schema_failed/.test(String(error));
  }
  assert.strictEqual(unsafeDenied, true, 'expected unsafe bridge paths to fail closed');

  const emitted = bridge.streamEmit({
    session_id: specialist.session_id,
    agentLabel: 'workflow-007-specialist',
    turn_id: 'workflow-007-turn',
    chunks: [
      { delimiter: 'agent_start', content: 'hello:' },
      { delimiter: 'agent_delta', content: 'governed-stream' },
      { delimiter: 'agent_end', content: '' },
    ],
    state_path: state,
  });
  assert.strictEqual(emitted.payload.chunk_count, 3);

  const rendered = bridge.streamRender({
    session_id: specialist.session_id,
    turn_id: 'workflow-007-turn',
    state_path: state,
  });
  assert(
    String(rendered.payload.rendered || '').includes('<agent:workflow-007-specialist:agent_start>hello:'),
    'expected rendered stream to preserve agent delimiters'
  );

  const turnRun = bridge.turnsRun({
    session_id: specialist.session_id,
    label: 'workflow-007-run',
    turns: [
      {
        message: 'draft governed answer',
        fail_first_attempt: true,
        recovery: 'retry_once',
      },
      {
        tool_name: 'context_patch',
        tool_args: {
          context: {
            recovered: true,
            answer_ready: true,
          },
          merge: true,
        },
      },
    ],
    state_path: state,
  });
  const runId = turnRun.payload.run.run_id;
  const shownRun = bridge.turnsShow({
    session_id: specialist.session_id,
    run_id: runId,
    state_path: state,
  });
  assert.strictEqual(shownRun.payload.run.status, 'completed');
  assert(
    shownRun.payload.run.turns.some((row) => row.status === 'error')
      && shownRun.payload.run.turns.some((row) => row.status === 'ok'),
    'expected turn run to record both transient error and recovered completion'
  );

  const network = bridge.networksCreate({
    session_id: coordinator.session_id,
    spec: {
      name: 'workflow-007-network',
      nodes: [
        { label: 'planner', role: 'planner', task: 'plan task', context: { lane: 'plan' } },
        { label: 'executor', role: 'executor', task: 'execute task', context: { lane: 'execute' } },
      ],
      edges: [
        {
          from: 'planner',
          to: 'executor',
          relation: 'handoff',
          importance: 0.7,
          auto_handoff: true,
          reason: 'planner_to_executor',
        },
      ],
    },
    state_path: state,
  });
  const networkId = network.payload.network.network_id;
  const networkState = bridge.networksStatus({
    session_id: coordinator.session_id,
    network_id: networkId,
    state_path: state,
  });
  assert.strictEqual(networkState.payload.network.network_id, networkId);
  assert.strictEqual(networkState.payload.network.nodes.length, 2);
  assert.strictEqual(networkState.payload.network.edges.length, 1);

  console.log(JSON.stringify({ ok: true, type: 'swarm_workflow_007_bridge_test' }));
}

run();
