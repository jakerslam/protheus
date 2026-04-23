#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.join(ROOT, 'client', 'runtime', 'lib', 'ts_entrypoint.ts');
const OPS = path.join(ROOT, 'client', 'runtime', 'systems', 'ops', 'run_infring_ops.ts');
const POLICY_PATH = path.join(ROOT, 'client', 'runtime', 'config', 'mcp_a2a_venom_contract_gate_policy.json');

function parseLastJson(stdout) {
  const text = String(stdout || '').trim();
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {}
  const lines = text.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let index = lines.length - 1; index >= 0; index -= 1) {
    const candidate = lines[index];
    if (!candidate.startsWith('{')) continue;
    try {
      return JSON.parse(candidate);
    } catch {}
  }
  return null;
}

function runOps(args) {
  const out = spawnSync(process.execPath, [ENTRYPOINT, OPS].concat(args), {
    cwd: ROOT,
    encoding: 'utf8',
    env: {
      ...process.env,
      INFRING_OPS_LOCAL_TIMEOUT_MS: '240000',
      INFRING_OPS_LOCAL_TIMEOUT_MS: '240000',
      INFRING_OPS_USE_PREBUILT: '0',
      INFRING_OPS_USE_PREBUILT: '0',
    },
  });
  return {
    status: Number.isFinite(Number(out.status)) ? Number(out.status) : 1,
    stdout: String(out.stdout || ''),
    stderr: String(out.stderr || ''),
    payload: parseLastJson(out.stdout),
  };
}

function main() {
  const policy = JSON.parse(fs.readFileSync(POLICY_PATH, 'utf8'));
  assert.equal(policy.fail_closed, true);
  assert.equal(policy.routes.mcp_discover.script, 'adapters/cognition/skills/mcp/mcp_gateway.ts');
  assert.deepStrictEqual(policy.routes.mcp_discover.args, [
    'discover',
    '--query=memory',
    '--risk-tier=2',
  ]);

  const canonical = runOps([
    'security-plane',
    'mcp-a2a-venom-contract-gate',
    '--boundary=conduit_only',
    '--strict=1',
  ]);
  assert.equal(canonical.status, 0, `canonical route failed\n${canonical.stderr}`);
  assert(canonical.payload, 'missing canonical payload');
  assert.equal(canonical.payload.type, 'security_plane_contract_lane');
  assert.equal(canonical.payload.contract_id, 'V6-SEC-015');
  assert.equal(canonical.payload.mode, 'mcp-a2a-venom-contract-gate');
  assert.equal(Array.isArray(canonical.payload.missing_flags), true);
  assert.equal(canonical.payload.missing_flags.length, 0);
  assert.equal(Array.isArray(canonical.payload.mismatch_flags), true);
  assert.equal(canonical.payload.mismatch_flags.length, 0);
  assert.equal(fs.existsSync(String(canonical.payload.state_path || '')), true);
  const canonicalState = JSON.parse(fs.readFileSync(canonical.payload.state_path, 'utf8'));
  assert.equal(canonicalState.provided_flags.boundary, 'conduit_only');

  const alias = runOps([
    'security-plane',
    'mcp_a2a_venom_contract_gate',
    '--boundary=conduit_only',
    '--strict=1',
  ]);
  assert.equal(alias.status, 0, `alias route failed\n${alias.stderr}`);
  assert(alias.payload, 'missing alias payload');
  assert.equal(alias.payload.type, 'security_plane_contract_lane');
  assert.equal(alias.payload.contract_id, 'V6-SEC-015');
  assert.equal(alias.payload.mode, 'mcp-a2a-venom-contract-gate');

  const mismatch = runOps([
    'security-plane',
    'mcp-a2a-venom-contract-gate',
    '--boundary=any',
    '--strict=1',
  ]);
  assert.equal(mismatch.status, 2, `expected fail-closed mismatch exit=2, got ${mismatch.status}`);
  assert(mismatch.payload, 'missing mismatch payload');
  assert.equal(mismatch.payload.ok, false);
  assert.deepStrictEqual(mismatch.payload.mismatch_flags, ['boundary:conduit_only']);

  console.log(JSON.stringify({ ok: true, type: 'mcp_a2a_venom_contract_gate_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
