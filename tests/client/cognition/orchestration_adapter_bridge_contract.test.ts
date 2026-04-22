#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../..');
const bridge = require(path.join(ROOT, 'adapters/runtime/orchestration_cognition_impl/core_bridge.ts'));
const shared = require(path.join(ROOT, 'adapters/runtime/orchestration_cognition_impl/cli_shared.ts'));

function main() {
  assert.strictEqual(
    shared.shouldFallbackForUnsupportedOp(
      { ok: false, reason_code: 'unsupported_op:taskgroup.list_agents.v2' },
      'taskgroup.list_agents',
    ),
    true,
  );
  assert.strictEqual(
    shared.shouldFallbackForUnsupportedOp(
      { ok: false, reason: 'unsupported_op:coordinator.status.runtime' },
      'coordinator.status',
    ),
    true,
  );
  assert.strictEqual(
    shared.shouldFallbackForUnsupportedOp(
      { ok: false, reason_code: 'timeout_failed' },
      'coordinator.timeout',
    ),
    false,
  );
  assert.strictEqual(
    shared.shouldFallbackForUnsupportedOp(
      { ok: true, reason_code: 'unsupported_op:coordinator.timeout' },
      'coordinator.timeout',
    ),
    false,
  );

  const normalizedError = bridge.normalizeBridgePayload({
    ok: false,
    type: 'ops_domain_spawn_error',
    reason: 'spawn_failed',
  });
  assert.strictEqual(normalizedError.ok, false);
  assert.strictEqual(normalizedError.reason_code, 'spawn_failed');
  assert.strictEqual(normalizedError.type, 'ops_domain_spawn_error');

  const passthroughPayload = bridge.normalizeBridgePayload({
    exists: true,
    file_path: '/tmp/scratchpad.json',
  });
  assert.strictEqual(passthroughPayload.exists, true);
  assert.strictEqual(passthroughPayload.file_path, '/tmp/scratchpad.json');

  let capturedArgs = null;
  let capturedOptions = null;
  const invokeOk = bridge.invokeOrchestrationWithBridge(
    'taskgroup.query',
    { task_group_id: 'abc-123' },
    { env: { SAMPLE_ENV: '1' } },
    (args, options) => {
      capturedArgs = args;
      capturedOptions = options;
      return { payload: { ok: true, type: 'orchestration_taskgroup_query', task_group_id: 'abc-123' } };
    },
  );
  assert.strictEqual(invokeOk.ok, true);
  assert.strictEqual(invokeOk.task_group_id, 'abc-123');
  assert.deepStrictEqual(capturedArgs.slice(0, 2), ['orchestration', 'invoke']);
  assert.strictEqual(capturedArgs[2], '--op=taskgroup.query');
  assert.ok(capturedArgs[3].startsWith('--payload-json='));
  assert.strictEqual(capturedOptions.unknownDomainFallback, false);
  assert.strictEqual(capturedOptions.env.SAMPLE_ENV, '1');

  const invokeFromStdout = bridge.invokeOrchestrationWithBridge(
    'coordinator.status',
    { task_id: 'task-1' },
    {},
    () => ({
      payload: null,
      stdout: 'prefix noise\n{"ok":true,"type":"orchestration_coordinator_status","task_id":"task-1"}\n',
      stderr: '',
      status: 0,
    }),
  );
  assert.strictEqual(invokeFromStdout.ok, true);
  assert.strictEqual(invokeFromStdout.task_id, 'task-1');

  const invokeNoBridge = bridge.invokeOrchestrationWithBridge(
    'taskgroup.query',
    { task_group_id: 'missing' },
    {},
    () => null,
  );
  assert.strictEqual(invokeNoBridge.ok, false);
  assert.strictEqual(invokeNoBridge.reason_code, 'bridge_unavailable');

  const invokeFailed = bridge.invokeOrchestrationWithBridge(
    'taskgroup.query',
    { task_group_id: 'missing' },
    {},
    () => ({ payload: null, stdout: '', stderr: 'boom', status: 7 }),
  );
  assert.strictEqual(invokeFailed.ok, false);
  assert.strictEqual(invokeFailed.reason_code, 'invoke_failed:7');
  assert.strictEqual(invokeFailed.stderr, 'boom');

  console.log(JSON.stringify({ ok: true, type: 'orchestration_adapter_bridge_contract_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
