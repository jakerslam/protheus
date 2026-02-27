#!/usr/bin/env node
'use strict';

const fs = require('fs');
const os = require('os');
const path = require('path');
const crypto = require('crypto');
const assert = require('assert');
const { spawnSync } = require('child_process');

function ensureDir(p) {
  if (!fs.existsSync(p)) fs.mkdirSync(p, { recursive: true });
}

function writeJson(filePath, value) {
  ensureDir(path.dirname(filePath));
  fs.writeFileSync(filePath, JSON.stringify(value, null, 2), 'utf8');
}

function parseJson(out) {
  const lines = String(out || '').trim().split('\n').map((row) => row.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {
      // continue
    }
  }
  return null;
}

function runNode(cwd, args, env = {}) {
  return spawnSync('node', args, {
    cwd,
    encoding: 'utf8',
    env: {
      ...process.env,
      ...env
    }
  });
}

function sha256File(filePath) {
  const body = fs.readFileSync(filePath);
  return crypto.createHash('sha256').update(body).digest('hex');
}

function attest(secret, nodeId, constitutionHash) {
  return crypto.createHmac('sha256', secret).update(`${nodeId}|${constitutionHash}`, 'utf8').digest('hex');
}

function run() {
  const repoRoot = path.resolve(__dirname, '..', '..', '..');
  const scriptPath = path.join(repoRoot, 'systems', 'hardware', 'attested_assimilation_plane.js');
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'attested-assimilation-'));

  const constitutionPath = path.join(tmp, 'AGENT-CONSTITUTION.md');
  fs.writeFileSync(constitutionPath, '# Constitution\nRoot\n', 'utf8');

  const policyPath = path.join(tmp, 'hardware_policy.json');
  writeJson(policyPath, {
    version: '1.0',
    constitution_path: constitutionPath,
    state_path: path.join(tmp, 'state', 'state.json'),
    audit_path: path.join(tmp, 'state', 'audit.jsonl'),
    required_attestation_secret_env: 'HARDWARE_ASSIMILATION_SECRET',
    idle_dormant_sec: 60,
    max_nodes: 16,
    scheduler: {
      default_lease_sec: 30,
      max_lease_sec: 120,
      max_leases_per_node: 2,
      work_steal_enabled: true
    }
  });

  const constitutionHash = sha256File(constitutionPath);
  const secret = 'test-secret';
  const nodeId = 'node_a';

  const badJoin = runNode(repoRoot, [
    scriptPath,
    'join',
    `--node-id=${nodeId}`,
    '--attestation=bad',
    `--constitution-hash=${constitutionHash}`,
    '--capabilities-json={"ram_gb":16,"cpu_threads":8}',
    `--policy=${policyPath}`
  ], {
    HARDWARE_ASSIMILATION_SECRET: secret
  });
  assert.strictEqual(badJoin.status, 1, 'bad attestation should fail join');

  const goodJoin = runNode(repoRoot, [
    scriptPath,
    'join',
    `--node-id=${nodeId}`,
    `--attestation=${attest(secret, nodeId, constitutionHash)}`,
    `--constitution-hash=${constitutionHash}`,
    '--capabilities-json={"ram_gb":16,"cpu_threads":8}',
    `--policy=${policyPath}`
  ], {
    HARDWARE_ASSIMILATION_SECRET: secret
  });
  assert.strictEqual(goodJoin.status, 0, goodJoin.stderr || 'good attestation should join');

  const hb = runNode(repoRoot, [scriptPath, 'heartbeat', `--node-id=${nodeId}`, `--policy=${policyPath}`]);
  assert.strictEqual(hb.status, 0, hb.stderr || 'heartbeat should pass');

  const schedule = runNode(repoRoot, [
    scriptPath,
    'schedule',
    '--work-id=work_1',
    '--required-ram-gb=8',
    '--required-cpu-threads=4',
    '--lease-sec=30',
    `--policy=${policyPath}`
  ]);
  assert.strictEqual(schedule.status, 0, schedule.stderr || 'schedule should pass');
  const schedulePayload = parseJson(schedule.stdout);
  assert.ok(schedulePayload.lease && schedulePayload.lease.lease_id, 'schedule should emit lease');

  const complete = runNode(repoRoot, [
    scriptPath,
    'complete',
    `--lease-id=${schedulePayload.lease.lease_id}`,
    `--policy=${policyPath}`
  ]);
  assert.strictEqual(complete.status, 0, complete.stderr || 'complete should pass');

  const status = runNode(repoRoot, [scriptPath, 'status', `--policy=${policyPath}`]);
  assert.strictEqual(status.status, 0, status.stderr || 'status should pass');
  const statusPayload = parseJson(status.stdout);
  assert.ok(statusPayload.nodes[nodeId], 'status should include joined node');

  console.log('attested_assimilation_plane.test.js: OK');
}

try {
  run();
} catch (err) {
  console.error(`attested_assimilation_plane.test.js: FAIL: ${err && err.stack ? err.stack : err.message}`);
  process.exit(1);
}
