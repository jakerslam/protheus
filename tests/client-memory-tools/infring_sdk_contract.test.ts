#!/usr/bin/env node
'use strict';

import assert from 'node:assert';
import fs from 'node:fs';
import {
  InfringSdkClient,
  PRODUCTION_TRANSPORT_SURFACE,
  createInMemoryTransport,
  createResidentIpcTransport,
  type InfringTransport,
  type InfringTransportRequest,
} from '../../packages/infring-sdk/src/index';

async function run(): Promise<void> {
  assert.equal(
    PRODUCTION_TRANSPORT_SURFACE,
    'resident_ipc_only',
    'production SDK transport surface should stay resident IPC only'
  );
  assert.equal(
    typeof createResidentIpcTransport,
    'function',
    'resident IPC transport should remain exported from the SDK surface'
  );
  const transportSource = fs.readFileSync(
    '/Users/jay/.openclaw/workspace/packages/infring-sdk/src/transports.ts',
    'utf8'
  );
  assert.ok(
    !transportSource.includes('cli_dev_only'),
    'production SDK transport surface must not re-export dev-only CLI transport'
  );

  const sdk = new InfringSdkClient({
    transport: createInMemoryTransport({
      submit_task: {
        task_id: 'task_seeded_submit',
        accepted: true,
        status: 'queued',
      },
      attach_policies: {
        applied_policy_refs: ['policy.alpha', 'policy.beta'],
      },
    }),
  });

  const submit = await sdk.submitTask({
    prompt: 'run regression contract',
  });
  assert.equal(submit.ok, true, 'submitTask should return ok envelope');
  assert.equal(submit.operation, 'submit_task', 'submitTask should map to submit_task');
  assert.equal(submit.data.status, 'queued', 'seeded in-memory submit status should be queued');

  const attach = await sdk.attachPolicies({
    mode: 'replace',
    policies: [{ policy_ref: 'policy.alpha' }, { policy_ref: 'policy.beta' }],
  });
  assert.equal(attach.ok, true, 'attachPolicies should return ok envelope');
  assert.deepEqual(
    sdk.getAttachedPolicyRefs(),
    ['policy.alpha', 'policy.beta'],
    'attached policy refs should persist on client'
  );

  const synthetic = createInMemoryTransport({}, { unseeded_behavior: 'synthetic_success' });
  const syntheticResult = await synthetic.invoke({
    operation: 'submit_task',
    payload: { prompt: 'synthetic fallback' },
    policy_refs: ['policy.synthetic'],
  });
  assert.equal(
    syntheticResult.ok,
    true,
    'synthetic unseeded behavior should return a successful envelope'
  );
  assert.equal(
    (syntheticResult.data as Record<string, unknown>).synthetic_fallback,
    true,
    'synthetic unseeded behavior should mark the fallback envelope'
  );
  assert.deepEqual(
    syntheticResult.receipts.map((row) => row.policy_ref),
    ['policy.synthetic', 'sdk.in_memory.synthetic_fallback'],
    'synthetic fallback should emit a synthetic receipt policy ref'
  );

  let captured: InfringTransportRequest | null = null;
  const spyTransport: InfringTransport = {
    async invoke(request) {
      captured = request;
      return {
        ok: true,
        operation: request.operation,
        trace_id: 'trace_test',
        receipts: [],
        data: {
          task_id: 'task_capture',
          accepted: true,
          status: 'queued',
        },
      };
    },
  };
  const spyClient = new InfringSdkClient({
    transport: spyTransport,
    default_policy_refs: ['policy.runtime.default'],
  });

  await spyClient.submitTask({
    prompt: 'capture policy refs',
    policy_refs: ['policy.inline.override'],
  });

  assert.ok(captured, 'transport should capture request');
  assert.deepEqual(
    captured && captured.policy_refs,
    ['policy.runtime.default', 'policy.inline.override'],
    'default and inline policy refs should both flow into transport call'
  );

  process.stdout.write(
    `${JSON.stringify({ ok: true, type: 'infring_sdk_contract_test' }, null, 2)}\n`
  );
}

run().catch((error) => {
  process.stderr.write(
    `${String(error && (error as Error).stack ? (error as Error).stack : error)}\n`
  );
  process.exit(1);
});
