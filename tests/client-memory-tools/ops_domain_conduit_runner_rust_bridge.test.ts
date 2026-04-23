#!/usr/bin/env node
'use strict';

const assert = require('assert');
const path = require('path');
require(path.resolve(__dirname, '..', '..', 'client', 'runtime', 'lib', 'ts_bootstrap.ts')).installTsRequireHook();

const mod = require(path.resolve(__dirname, '..', '..', 'client', 'runtime', 'lib', 'ops_domain_conduit_runner.ts'));

function run() {
  process.env.INFRING_OPS_USE_PREBUILT = '0';
  process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = '120000';
  process.env.INFRING_OPS_DOMAIN_SKIP_RUNTIME_GATE = '0';
  process.env.INFRING_OPS_DOMAIN_STDIO_TIMEOUT_MS = '34567';
  delete process.env.INFRING_OPS_DOMAIN_BRIDGE_TIMEOUT_MS;
  delete process.env.INFRING_CONDUIT_BRIDGE_TIMEOUT_MS;

  const parsed = mod.parseArgs(['--domain', 'legacy-retired-lane', 'run', '--lane-id=FOO-3']);
  assert.equal(parsed.domain, 'legacy-retired-lane');
  assert.deepStrictEqual(parsed._, ['run']);

  const options = mod.buildRunOptions(parsed);
  assert.equal(options.skipRuntimeGate, false);
  assert.equal(options.stdioTimeoutMs, 34567);
  assert.equal(options.timeoutMs, 125000);
  assert.equal(options.runContext, null);
}

run();
console.log(JSON.stringify({ ok: true, type: 'ops_domain_conduit_runner_rust_bridge_test' }));
