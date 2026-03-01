#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const BIN_PROTHEUS = path.join(ROOT, 'bin', 'protheus');
const BIN_PROTHEUSCTL = path.join(ROOT, 'bin', 'protheusctl');
const BIN_PROTHEUSD = path.join(ROOT, 'bin', 'protheusd');
const BIN_PROTHEUS_TOP = path.join(ROOT, 'bin', 'protheus-top');

function writeJson(filePath, payload) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function parseJson(stdout) {
  const text = String(stdout || '').trim();
  if (!text) return null;
  try { return JSON.parse(text); } catch {}
  const lines = text.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try { return JSON.parse(lines[i]); } catch {}
  }
  return null;
}

function runBin(binPath, args) {
  const run = spawnSync(binPath, args, {
    cwd: ROOT,
    encoding: 'utf8'
  });
  return {
    status: Number.isFinite(run.status) ? run.status : 1,
    stdout: String(run.stdout || ''),
    stderr: String(run.stderr || ''),
    payload: parseJson(run.stdout)
  };
}

try {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'protheus-cli-entrypoints-'));
  const stateRoot = path.join(tmp, 'state');
  const policyPath = path.join(tmp, 'protheus_control_plane_policy.json');

  writeJson(policyPath, {
    enabled: true,
    shadow_only: true,
    paths: {
      state_root: stateRoot,
      daemon_path: path.join(stateRoot, 'daemon.json'),
      commands_path: path.join(stateRoot, 'commands.jsonl'),
      jobs_path: path.join(stateRoot, 'jobs.json'),
      incidents_path: path.join(stateRoot, 'incidents.jsonl'),
      release_path: path.join(stateRoot, 'release.json'),
      registry_path: path.join(stateRoot, 'registry.json'),
      latest_path: path.join(stateRoot, 'latest.json'),
      receipts_path: path.join(stateRoot, 'receipts.jsonl'),
      auth_sources_path: path.join(stateRoot, 'auth_sources.json'),
      integrity_queue_path: path.join(stateRoot, 'integrity_queue.json'),
      event_ledger_path: path.join(stateRoot, 'events.jsonl'),
      routing_preflight_path: path.join(stateRoot, 'preflight.json'),
      routing_doctor_path: path.join(stateRoot, 'doctor.json'),
      routing_health_path: path.join(stateRoot, 'health.json'),
      warm_snapshot_path: path.join(stateRoot, 'warm_snapshot.json'),
      benchmark_state_path: path.join(stateRoot, 'benchmark.json')
    }
  });

  let out = runBin(BIN_PROTHEUSD, ['start', `--policy=${policyPath}`]);
  assert.strictEqual(out.status, 0, out.stderr || out.stdout);
  assert.ok(out.payload && out.payload.type === 'protheus_daemon_control', 'start should return daemon control receipt');

  out = runBin(BIN_PROTHEUS, ['status', `--policy=${policyPath}`]);
  assert.strictEqual(out.status, 0, out.stderr || out.stdout);
  assert.ok(out.payload && out.payload.type === 'protheus_control_plane_status', 'status should return status payload');
  assert.strictEqual(out.payload.daemon.running, true, 'daemon should be running after start');

  out = runBin(BIN_PROTHEUSCTL, ['job-submit', '--kind=cli_test', `--policy=${policyPath}`]);
  assert.strictEqual(out.status, 0, out.stderr || out.stdout);
  assert.ok(out.payload && out.payload.type === 'protheus_job_submit', 'job-submit should return receipt');

  out = runBin(BIN_PROTHEUSD, ['tick', '--max=1', `--policy=${policyPath}`]);
  assert.strictEqual(out.status, 0, out.stderr || out.stdout);
  assert.ok(out.payload && out.payload.type === 'protheus_job_runner_tick', 'tick should route to job-runner');

  out = runBin(BIN_PROTHEUS_TOP, [`--policy=${policyPath}`]);
  assert.strictEqual(out.status, 0, out.stderr || out.stdout);
  assert.ok(out.payload && out.payload.type === 'protheus_top', 'top wrapper should return top payload');

  out = runBin(BIN_PROTHEUSD, ['stop', `--policy=${policyPath}`]);
  assert.strictEqual(out.status, 0, out.stderr || out.stdout);
  assert.ok(out.payload && out.payload.type === 'protheus_daemon_control', 'stop should return daemon control receipt');

  fs.rmSync(tmp, { recursive: true, force: true });
  console.log('protheus_cli_entrypoints.test.js: OK');
} catch (err) {
  console.error(`protheus_cli_entrypoints.test.js: FAIL: ${err.message}`);
  process.exit(1);
}
