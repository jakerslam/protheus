#!/usr/bin/env node
'use strict';

import assert from 'node:assert';
import {
  InfringSdkClient,
  createInMemoryTransport,
  type InfringTransport,
  type InfringTransportRequest,
} from '../../packages/infring-sdk/src/index';

async function run(): Promise<void> {
  const sdk = new InfringSdkClient({
    transport: createInMemoryTransport(),
  });

  const submit = await sdk.submitTask({
    prompt: 'run regression contract',
  });
  assert.equal(submit.ok, true, 'submitTask should return ok envelope');
  assert.equal(submit.operation, 'submit_task', 'submitTask should map to submit_task');
  assert.equal(submit.data.status, 'queued', 'default in-memory submit status should be queued');

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
