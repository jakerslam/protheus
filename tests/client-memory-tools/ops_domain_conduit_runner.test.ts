#!/usr/bin/env node
'use strict';

const assert = require('assert');
const path = require('path');

require(path.resolve(__dirname, '..', '..', 'client', 'runtime', 'lib', 'ts_bootstrap.ts')).installTsRequireHook();

const runner = require(path.resolve(__dirname, '..', '..', 'client', 'runtime', 'lib', 'ops_domain_conduit_runner.ts'));
const spine = require(path.resolve(__dirname, '..', '..', 'client', 'runtime', 'lib', 'spine_conduit_bridge.ts'));
const conduit = require(path.resolve(__dirname, '..', '..', 'client', 'runtime', 'lib', 'direct_conduit_lane_bridge.ts'));

async function run() {
  const missingDomain = await runner.run([]);
  assert.equal(missingDomain.status, 2);
  assert.equal(missingDomain.payload.ok, false);
  assert.equal(missingDomain.payload.type, 'ops_domain_conduit_bridge_error');
  assert.equal(missingDomain.payload.reason, 'missing_domain');
  assert.equal(missingDomain.payload.routed_via, 'core_local');

  const missingOpsDomain = await spine.runOpsDomainCommand('', ['status']);
  assert.equal(missingOpsDomain.ok, false);
  assert.equal(missingOpsDomain.status, 1);
  assert.equal(missingOpsDomain.payload.reason, 'missing_domain');
  assert.deepStrictEqual(missingOpsDomain.detail, missingOpsDomain.payload);

  const missingLane = await conduit.runLaneViaConduit('', process.cwd());
  assert.equal(missingLane.ok, false);
  assert.equal(missingLane.type, 'conduit_lane_bridge_error');
  assert.equal(missingLane.error, 'missing_lane_id');
}

run()
  .then(() => {
    console.log(JSON.stringify({ ok: true, type: 'ops_domain_conduit_runner_test' }));
  })
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
