#!/usr/bin/env node
'use strict';

// SRS coverage: V6-WORKFLOW-007.1, V6-WORKFLOW-007.2, V6-WORKFLOW-007.3,
// V6-WORKFLOW-007.4, V6-WORKFLOW-007.5, V6-WORKFLOW-007.6, V6-WORKFLOW-007.7

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');

const bridge = require('../../client/runtime/systems/autonomy/swarm_sessions_bridge.ts');

function run() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'swarm-sessions-bridge-'));
  const state = path.join(tmpDir, 'state.json');

  // Test 2 parity: recursive decomposition with explicit parent-child lineage.
  const root = bridge.sessionsSpawn({
    task: 'root recursive task',
    state_path: state,
  });
  const level1 = bridge.sessionsSpawn({
    task: 'spawn level1 child',
    session_id: root.session_id,
    state_path: state,
  });
  const level2 = bridge.sessionsSpawn({
    task: 'spawn level2 child',
    session_id: level1.session_id,
    state_path: state,
  });
  const level1State = bridge.sessionsState({
    session_id: level1.session_id,
    state_path: state,
  });
  assert(
    Array.isArray(level1.tool_access) && level1.tool_access.includes('sessions_send'),
    'expected spawned sessions to advertise sessions_send in tool_access'
  );
  assert(
    level1.tool_manifest && Array.isArray(level1.tool_manifest.tool_access),
    'expected spawned sessions to expose an authoritative tool manifest'
  );
  assert(
    level1.agent_bootstrap && level1.agent_bootstrap.version === 'swarm-agent-bootstrap/v1',
    'expected spawned sessions to expose a generic-agent bootstrap contract'
  );
  assert(
    String(level1.agent_bootstrap.prompt || '').includes('Use direct swarm bridge commands'),
    'expected bootstrap prompt to direct agents toward bridge commands'
  );
  assert.strictEqual(level2.payload.payload.parent_id, level1.session_id);
  assert(
    Array.isArray(level1State.payload.session.children)
      && level1State.payload.session.children.includes(level2.session_id),
    'expected level1 to track spawned child lineage'
  );

  // Test 3 parity: byzantine mode in test context.
  const byzantine = bridge.sessionsSpawn({
    task: 'calculate 2+2',
    testMode: 'byzantine',
    faultPattern: JSON.stringify({ type: 'corruption', value: '2+2=5' }),
    state_path: state,
  });
  const byzantineState = bridge.sessionsState({
    session_id: byzantine.session_id,
    state_path: state,
  });
  assert.strictEqual(byzantineState.payload.session.byzantine, true);
  assert.strictEqual(
    String(byzantineState.payload.session.corruption_type || '').length > 0,
    true,
    'expected corruption_type to be present in byzantine mode'
  );

  // Test 5 parity: persistent sessions survive tick/check-in cycles.
  const persistent = bridge.sessionsSpawn({
    task: 'monitor and report',
    sessionType: 'persistent',
    ttlMinutes: 5,
    checkpointInterval: 1,
    state_path: state,
  });
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 1300);
  const persistentState = bridge.sessionsState({
    session_id: persistent.session_id,
    timeline: 1,
    state_path: state,
  });
  assert.strictEqual(Boolean(persistentState.payload.session.persistent), true);
  assert.strictEqual(
    persistentState.payload.session.status === 'persistent_running'
      || persistentState.payload.session.status === 'running',
    true,
    'expected persistent session to remain active after tick'
  );

  // Test 6 parity: direct inter-agent messaging with delivery + ack.
  const commParent = bridge.sessionsSpawn({
    task: 'communication parent',
    state_path: state,
  });
  const sender = bridge.sessionsSpawn({
    task: 'sender',
    session_id: commParent.session_id,
    state_path: state,
  });
  const receiver = bridge.sessionsSpawn({
    task: 'receiver',
    session_id: commParent.session_id,
    state_path: state,
  });
  const send = bridge.sessionsSend({
    sender: sender.session_id,
    session_id: receiver.session_id,
    message: 'process:[1,2,3]',
    delivery: 'at_least_once',
    state_path: state,
  });
  assert.strictEqual(Boolean(send.message_id), true);
  const inbox = bridge.sessionsReceive({
    session_id: receiver.session_id,
    limit: 8,
    state_path: state,
  });
  assert(inbox.message_count >= 1, 'expected receiver inbox to contain messages');
  const message = inbox.messages.find((row) => row.message_id === send.message_id);
  assert(message, 'expected sent message to be receivable by target session');
  const ack = bridge.sessionsAck({
    session_id: receiver.session_id,
    message_id: send.message_id,
    state_path: state,
  });
  assert.strictEqual(ack.payload.acknowledged, true);

  // Parent/child lineage-adjacent messaging should work without sibling-only routing.
  const directiveParent = bridge.sessionsSpawn({
    task: 'directive parent',
    state_path: state,
  });
  const directiveChild = bridge.sessionsSpawn({
    task: 'directive child',
    session_id: directiveParent.session_id,
    state_path: state,
  });
  const parentToChild = bridge.sessionsSend({
    sender: directiveParent.session_id,
    session_id: directiveChild.session_id,
    message: 'directive:inspect workspace root',
    delivery: 'at_least_once',
    state_path: state,
  });
  const childInbox = bridge.sessionsReceive({
    session_id: directiveChild.session_id,
    limit: 8,
    state_path: state,
  });
  assert(
    childInbox.messages.some((row) => row.message_id === parentToChild.message_id),
    'expected child to receive parent directive message'
  );
  const childToParent = bridge.sessionsSend({
    sender: directiveChild.session_id,
    session_id: directiveParent.session_id,
    message: 'directive_ack:workspace root inspected',
    delivery: 'at_least_once',
    state_path: state,
  });
  const parentInbox = bridge.sessionsReceive({
    session_id: directiveParent.session_id,
    limit: 8,
    state_path: state,
  });
  assert(
    parentInbox.messages.some((row) => row.message_id === childToParent.message_id),
    'expected parent to receive child acknowledgement message'
  );

  // Hierarchical budget reservation + settlement.
  const budgetParent = bridge.sessionsSpawn({
    task: 'budget-parent',
    max_tokens: 500,
    on_budget_exhausted: 'fail',
    state_path: state,
  });
  const budgetChild = bridge.sessionsSpawn({
    task: 'budget-child',
    session_id: budgetParent.session_id,
    max_tokens: 200,
    on_budget_exhausted: 'fail',
    state_path: state,
  });
  const budgetChildState = bridge.sessionsState({
    session_id: budgetChild.session_id,
    state_path: state,
  });
  const budgetParentState = bridge.sessionsState({
    session_id: budgetParent.session_id,
    state_path: state,
  });
  assert.strictEqual(
    budgetChildState.payload.session.budget_parent_session_id,
    budgetParent.session_id,
    'expected child session to record hierarchical budget parent'
  );
  assert(
    Number(budgetParentState.payload.session.budget.settled_child_tokens || 0) > 0,
    'expected parent budget to settle child token usage'
  );
  const bootstrap = bridge.sessionsBootstrap({
    session_id: budgetChild.session_id,
    state_path: state,
  });
  assert(
    bootstrap.bootstrap && bootstrap.bootstrap.commands && bootstrap.bootstrap.commands.sessions_send,
    'expected sessions_bootstrap to expose direct inter-agent messaging commands'
  );
  assert.strictEqual(
    bootstrap.bootstrap.budget.on_budget_exhausted,
    'fail',
    'expected sessions_bootstrap to surface fail-closed budget policy'
  );
  assert(
    Number(bootstrap.bootstrap.budget.remaining_tokens || 0) <= 200,
    'expected sessions_bootstrap to surface remaining budget telemetry'
  );

  // Test 7 parity: service discovery + result query.
  bridge.sessionsSpawn({
    task: 'calc-fast',
    role: 'calculator',
    agentLabel: 'swarm-test-7-calc-fast',
    auto_publish_results: 1,
    state_path: state,
  });
  bridge.sessionsSpawn({
    task: 'calc-thorough',
    role: 'calculator',
    agentLabel: 'swarm-test-7-calc-thorough',
    auto_publish_results: 1,
    state_path: state,
  });
  const query = bridge.sessionsQuery({
    agentRole: 'calculator',
    agentLabel: 'swarm-test-7-calc-*',
    wait: 1,
    min_count: 2,
    timeout_sec: 10,
    state_path: state,
  });
  assert(query.result_count >= 2, 'expected calculator result registry entries');
  assert(
    query.discovery
      && Array.isArray(query.discovery.instances)
      && query.discovery.instances.length >= 2,
    'expected role discovery to return active calculator instances'
  );

  // Test 4 parity: hard token budget enforcement, not advisory only.
  let hardBudgetRejected = false;
  try {
    bridge.sessionsSpawn({
      task: 'summarize largest programming language communities',
      max_tokens: 80,
      on_budget_exhausted: 'fail',
      state_path: state,
    });
  } catch (err) {
    hardBudgetRejected = /token_budget_exceeded/.test(String(err && err.message));
  }
  assert.strictEqual(hardBudgetRejected, true, 'expected hard budget fail-close rejection');

  // Workflow-007 smoke: handoff/context/tool/stream/turns/network surfaces remain live on the
  // canonical swarm bridge regression path as well as the dedicated workflow-007 tests.
  const workflowCoordinator = bridge.sessionsSpawn({
    task: 'workflow-007 bridge smoke coordinator',
    max_tokens: 1024,
    on_budget_exhausted: 'fail',
    state_path: state,
  });
  const workflowSpecialist = bridge.sessionsSpawn({
    task: 'workflow-007 bridge smoke specialist',
    session_id: workflowCoordinator.session_id,
    max_tokens: 192,
    on_budget_exhausted: 'fail',
    state_path: state,
  });
  const workflowContext = bridge.sessionsContextPut({
    session_id: workflowCoordinator.session_id,
    context: {
      objective: 'workflow-007 bridge smoke',
      oversized: 'x'.repeat(9000),
    },
    state_path: state,
  });
  assert.strictEqual(
    workflowContext.payload.receipt.degraded_mode,
    'context_compacted',
    'expected workflow-007 context propagation to compact under budget pressure'
  );
  const workflowHandoff = bridge.sessionsHandoff({
    session_id: workflowCoordinator.session_id,
    target_session_id: workflowSpecialist.session_id,
    reason: 'workflow-007 bridge smoke delegation',
    importance: 0.8,
    context: { delegated_goal: 'complete bridge smoke' },
    state_path: state,
  });
  assert.strictEqual(Boolean(workflowHandoff.payload.handoff.handoff_id), true);
  bridge.toolsRegisterJsonSchema({
    session_id: workflowSpecialist.session_id,
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
    state_path: state,
  });
  const workflowTool = bridge.toolsInvoke({
    session_id: workflowSpecialist.session_id,
    toolName: 'context_patch',
    args: {
      context: { tool_applied: true },
      merge: true,
    },
    state_path: state,
  });
  assert.strictEqual(workflowTool.payload.result.receipt.context.tool_applied, true);
  bridge.streamEmit({
    session_id: workflowSpecialist.session_id,
    turn_id: 'workflow-007-bridge-smoke-turn',
    agentLabel: 'workflow-007-bridge-smoke',
    chunks: [
      { delimiter: 'agent_start', content: 'hello:' },
      { delimiter: 'agent_delta', content: 'smoke' },
      { delimiter: 'agent_end', content: '' },
    ],
    state_path: state,
  });
  const workflowRender = bridge.streamRender({
    session_id: workflowSpecialist.session_id,
    turn_id: 'workflow-007-bridge-smoke-turn',
    state_path: state,
  });
  assert(
    String(workflowRender.payload.rendered || '').includes('workflow-007-bridge-smoke'),
    'expected workflow-007 stream render to preserve agent delimiters'
  );
  const workflowRun = bridge.turnsRun({
    session_id: workflowSpecialist.session_id,
    label: 'workflow-007-bridge-smoke',
    turns: [{ message: 'bridge smoke run', fail_first_attempt: true, recovery: 'retry_once' }],
    state_path: state,
  });
  const workflowRunState = bridge.turnsShow({
    session_id: workflowSpecialist.session_id,
    run_id: workflowRun.payload.run.run_id,
    state_path: state,
  });
  assert.strictEqual(workflowRunState.payload.run.status, 'completed');
  const workflowNetwork = bridge.networksCreate({
    session_id: workflowCoordinator.session_id,
    spec: {
      name: 'workflow-007-bridge-smoke-network',
      nodes: [
        { label: 'planner', role: 'planner', task: 'plan' },
        { label: 'executor', role: 'executor', task: 'execute' },
      ],
      edges: [
        { from: 'planner', to: 'executor', relation: 'handoff', importance: 0.7, auto_handoff: true },
      ],
    },
    state_path: state,
  });
  const workflowNetworkState = bridge.networksStatus({
    session_id: workflowCoordinator.session_id,
    network_id: workflowNetwork.payload.network.network_id,
    state_path: state,
  });
  assert.strictEqual(workflowNetworkState.payload.network.nodes.length, 2);

  // Dead-letter expiry + retry recovery.
  const dlqParent = bridge.sessionsSpawn({ task: 'dlq-parent', state_path: state });
  const dlqSender = bridge.sessionsSpawn({
    task: 'dlq-sender',
    session_id: dlqParent.session_id,
    state_path: state,
  });
  const dlqReceiver = bridge.sessionsSpawn({
    task: 'dlq-receiver',
    session_id: dlqParent.session_id,
    state_path: state,
  });
  bridge.sessionsSend({
    sender: dlqSender.session_id,
    session_id: dlqReceiver.session_id,
    message: 'expire-me',
    delivery: 'at_least_once',
    ttl_ms: 1,
    state_path: state,
  });
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 10);
  bridge.sessionsState({ session_id: dlqReceiver.session_id, state_path: state });
  const deadLetters = bridge.sessionsDeadLetters({
    session_id: dlqReceiver.session_id,
    state_path: state,
  });
  assert(deadLetters.payload.dead_letter_count >= 1, 'expected a dead-lettered message');
  const deadLetterMessageId = deadLetters.payload.dead_letters[0].message.message_id;
  const retried = bridge.sessionsRetryDeadLetter({
    message_id: deadLetterMessageId,
    state_path: state,
  });
  const recoveredInbox = bridge.sessionsReceive({
    session_id: dlqReceiver.session_id,
    limit: 8,
    state_path: state,
  });
  assert(
    recoveredInbox.messages.some((row) => row.message_id === retried.payload.retry_result.message_id),
    'expected retried dead-letter message to return to receiver inbox'
  );

  // Persistent resume / restart recovery command.
  const resumed = bridge.sessionsResume({
    session_id: persistent.session_id,
    state_path: state,
  });
  assert.strictEqual(resumed.payload.status, 'persistent_running');
}

run();
console.log(
  JSON.stringify({
    ok: true,
    type: 'swarm_sessions_bridge_test',
  })
);
