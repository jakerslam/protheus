#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.join(ROOT, 'client', 'runtime', 'lib', 'ts_entrypoint.ts');
const RUNNER = path.join(ROOT, 'tests', 'tooling', 'scripts', 'ci', 'srs_repair_lane_runner.ts');
const RECEIPT = path.join(ROOT, 'local', 'state', 'ops', 'srs_contract_runtime', 'V3-RACE-038A', 'latest.json');
const POLICY = JSON.parse(fs.readFileSync(path.join(ROOT, 'client', 'runtime', 'config', 'inter_infring_federation_trust_web_policy.json'), 'utf8'));

const proc = spawnSync(process.execPath, [ENTRYPOINT, RUNNER, '--id=V3-RACE-038A', '--strict=1'], {
  cwd: ROOT,
  encoding: 'utf8'
});
assert.equal(proc.status, 0, proc.stderr || proc.stdout);
assert.equal(fs.existsSync(RECEIPT), true, 'missing federation trust receipt');

const receipt = JSON.parse(fs.readFileSync(RECEIPT, 'utf8'));
assert.equal(receipt.ok, true);
assert.equal(receipt.id, 'V3-RACE-038A');
assert.equal(receipt.type, 'srs_contract_runtime_receipt');
assert.equal(Array.isArray(POLICY.checks), true);
assert.equal(POLICY.checks.length >= 4, true);
assert.match(POLICY.paths.latest_path, /inter_infring_federation_trust_web\/latest\.json$/);

console.log(JSON.stringify({ ok: true, type: 'inter_infring_federation_trust_web_test' }));
