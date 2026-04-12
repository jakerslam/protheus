#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.join(ROOT, 'client', 'runtime', 'lib', 'ts_entrypoint.ts');
const RUNNER = path.join(ROOT, 'tests', 'tooling', 'scripts', 'ci', 'srs_repair_lane_runner.ts');
const RECEIPT = path.join(ROOT, 'local', 'state', 'ops', 'srs_contract_runtime', 'V6-BROWSER-007', 'latest.json');
const POLICIES = [
  'client/runtime/config/browser/native_browser_daemon_policy.json',
  'client/runtime/config/browser/native_browser_cdp_policy.json',
  'client/runtime/config/browser/browser_session_vault_policy.json',
  'client/runtime/config/browser/browser_snapshot_refs_policy.json',
  'client/runtime/config/browser/browser_policy_gate_policy.json',
  'client/runtime/config/browser/browser_text_diff_policy.json',
  'client/runtime/config/browser/browser_cli_shadow_bridge_policy.json',
];

for (const rel of POLICIES) {
  assert.equal(fs.existsSync(path.join(ROOT, rel)), true, `missing browser policy: ${rel}`);
}

const proc = spawnSync(process.execPath, [ENTRYPOINT, RUNNER, '--id=V6-BROWSER-007', '--strict=1'], {
  cwd: ROOT,
  encoding: 'utf8'
});
assert.equal(proc.status, 0, proc.stderr || proc.stdout);
assert.equal(fs.existsSync(RECEIPT), true, 'missing browser lane receipt');

const receipt = JSON.parse(fs.readFileSync(RECEIPT, 'utf8'));
assert.equal(receipt.ok, true);
assert.equal(receipt.id, 'V6-BROWSER-007');
assert.equal(receipt.type, 'srs_contract_runtime_receipt');

console.log(JSON.stringify({ ok: true, type: 'browser_next10_bundle_test' }));
