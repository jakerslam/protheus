#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');

const { invokeKernel, invokeKernelPayload } = require('../../client/runtime/lib/protheus_kernel_bridge.ts');

function main() {
  process.env.PROTHEUS_OPS_USE_PREBUILT = '0';
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = '120000';

  const status = invokeKernel('memory-policy-kernel', 'status', {});
  assert.equal(status.ok, true);
  assert(status.payload && typeof status.payload === 'object', 'expected memory-policy status payload');

  const severity = invokeKernelPayload('memory-policy-kernel', 'severity-rank', { value: 'critical' });
  assert(Number.isFinite(Number(severity.rank)), 'expected severity-rank payload to include rank');

  console.log(JSON.stringify({ ok: true, type: 'protheus_kernel_bridge_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
