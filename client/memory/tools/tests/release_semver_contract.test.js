#!/usr/bin/env node
'use strict';

const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCRIPT = path.join(ROOT, 'systems', 'ops', 'release_semver_contract.js');

function run(args) {
  return spawnSync(process.execPath, [SCRIPT, ...args], {
    cwd: ROOT,
    encoding: 'utf8'
  });
}

function parseJson(raw) {
  try {
    return JSON.parse(String(raw || '').trim());
  } catch {
    return null;
  }
}

function fail(message) {
  console.error(`release_semver_contract.test.js FAILED: ${message}`);
  process.exit(1);
}

function assert(condition, message) {
  if (!condition) fail(message);
}

function main() {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'release-semver-contract-'));
  const planPath = path.join(tempDir, 'plan.json');
  const changelogPath = path.join(tempDir, 'changelog.md');

  const runRes = run(['run', '--strict=0', '--write=1', `--plan=${planPath}`, `--changelog=${changelogPath}`]);
  assert(runRes.status === 0, `run should pass: ${runRes.stderr}`);
  const payload = parseJson(runRes.stdout);
  assert(payload && payload.schema_id === 'release_semver_contract_result', 'expected schema_id');
  assert(['major', 'minor', 'patch', 'none'].includes(payload.bump), 'unexpected bump classification');
  assert(fs.existsSync(planPath), 'plan should be written');
  assert(fs.existsSync(changelogPath), 'changelog should be written');

  const statusRes = run(['status', `--plan=${planPath}`]);
  assert(statusRes.status === 0, `status should pass: ${statusRes.stderr}`);
  const statusPayload = parseJson(statusRes.stdout);
  assert(statusPayload && statusPayload.schema_id === 'release_semver_contract_result', 'status should return plan');

  console.log('release_semver_contract.test.js: OK');
}

main();
