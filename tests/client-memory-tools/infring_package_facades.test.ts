#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');

require('../../client/runtime/lib/ts_bootstrap.ts').installTsRequireHook();

const core = require('../../packages/infring-core/index.ts');
const edge = require('../../packages/infring-edge/index.ts');

function main() {
  process.env.INFRING_OPS_USE_PREBUILT = '0';
  process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = '120000';

  const spine = core.spineStatus();
  assert.equal(spine.ok, true);
  assert.equal(String(spine.payload && spine.payload.type), 'spine_status');

  const reflex = core.reflexStatus();
  assert.equal(reflex.ok, true);
  assert(reflex.payload && typeof reflex.payload === 'object', 'expected reflex payload');

  const gate = core.gateStatus();
  assert.equal(gate.ok, true);
  assert.equal(String(gate.payload && gate.payload.type), 'security_plane_status');

  const edgeStatus = edge.edgeRuntime('status');
  assert.equal(edgeStatus.ok, true);

  const lifecycle = edge.edgeLifecycle('status');
  assert.equal(lifecycle.ok, true);

  const wrappers = edge.edgeWrapper('status', { target: 'android_termux' });
  assert.equal(wrappers.ok, true);
  assert.equal(Boolean(wrappers.payload && wrappers.payload.policy_present), true);

  const swarmNotice = edge.edgeSwarm('status');
  assert.equal(swarmNotice.ok, false);
  assert.equal(Boolean(swarmNotice.payload && swarmNotice.payload.deprecated), true);

  const bundle = edge.edgeStatusBundle({ target: 'android_termux' });
  assert.equal(bundle.ok, true);
  assert(Array.isArray(bundle.supported_surface), 'expected supported surface summary');

  console.log(JSON.stringify({ ok: true, type: 'infring_package_facades_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
