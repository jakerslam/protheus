#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCRIPT = path.join(ROOT, 'systems', 'security', 'venom_containment_layer.js');

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function run(args) {
  const r = spawnSync('node', [SCRIPT, ...args], {
    cwd: ROOT,
    encoding: 'utf8'
  });
  let payload = null;
  try { payload = JSON.parse(String(r.stdout || '').trim()); } catch {}
  return {
    status: r.status == null ? 1 : r.status,
    payload,
    stderr: String(r.stderr || '')
  };
}

try {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'venom-layer-'));
  const policyPath = path.join(tmp, 'config', 'venom_containment_policy.json');

  const stateRoot = path.join(tmp, 'state', 'security', 'venom_containment');
  const startupAttPath = path.join(tmp, 'state', 'security', 'startup_attestation.json');
  const soulPath = path.join(tmp, 'state', 'security', 'soul_token_guard.json');
  const leasePath = path.join(tmp, 'state', 'security', 'capability_leases.json');
  const masterQueue = path.join(tmp, 'state', 'workflow', 'learning_conduit', 'master_training_queue.jsonl');

  writeJson(startupAttPath, {
    type: 'startup_attestation',
    signature: 'sig_ok',
    expires_at: new Date(Date.now() + 24 * 60 * 60 * 1000).toISOString()
  });
  writeJson(soulPath, {
    fingerprint: 'fp_test_1',
    token: 'soul_test_1'
  });
  writeJson(leasePath, {
    active: []
  });

  writeJson(policyPath, {
    version: '1.0',
    enabled: true,
    shadow_only: true,
    defensive_only_invariant: true,
    staged_ramp: {
      tease_actions: 2,
      challenge_actions: 3,
      degrade_actions: 5,
      lockout_actions: 7,
      lockout_cooldown_minutes: 10
    },
    paths: {
      state_root: stateRoot,
      sessions_path: path.join(stateRoot, 'sessions.json'),
      latest_path: path.join(stateRoot, 'latest.json'),
      history_path: path.join(stateRoot, 'history.jsonl'),
      profiles_path: path.join(stateRoot, 'profiles.json'),
      startup_attestation_path: startupAttPath,
      soul_token_guard_path: soulPath,
      lease_state_path: leasePath,
      master_queue_path: masterQueue
    },
    forensics: {
      enabled: true,
      include_watermark: true,
      master_conduit_mirror: true,
      evidence_dir: path.join(stateRoot, 'evidence'),
      events_path: path.join(stateRoot, 'forensic_events.jsonl')
    }
  });

  let res = run(['evaluate', `--policy=${policyPath}`, '--session-id=auth_ok', '--source=local', '--action=run', '--risk=low']);
  assert.strictEqual(res.status, 0, `authorized evaluate should pass: ${res.stderr}`);
  assert.ok(res.payload && res.payload.ok === true, 'payload should be ok');
  assert.strictEqual(res.payload.unauthorized, false, 'authorized run should not be unauthorized');
  assert.strictEqual(String(res.payload.stage || ''), 'none', 'authorized run should be at stage none');

  const stages = [];
  for (let i = 0; i < 8; i += 1) {
    res = run([
      'evaluate',
      `--policy=${policyPath}`,
      '--session-id=bad_copy_1',
      '--source=webhook',
      '--action=deploy',
      '--risk=high',
      '--runtime-class=gpu_heavy',
      '--unauthorized=1'
    ]);
    assert.strictEqual(res.status, 0, `unauthorized evaluate run ${i} should pass: ${res.stderr}`);
    assert.ok(res.payload && res.payload.ok === true, 'unauthorized payload should be ok');
    stages.push(String(res.payload.stage || ''));
  }

  assert.ok(stages.includes('tease'), 'stages should include tease');
  assert.ok(stages.includes('challenge'), 'stages should include challenge');
  assert.ok(stages.includes('degrade') || stages.includes('lockout'), 'stages should degrade/lockout');
  assert.strictEqual(stages[stages.length - 1], 'lockout', 'final stage should reach lockout');

  const forensicEventsPath = path.join(stateRoot, 'forensic_events.jsonl');
  assert.ok(fs.existsSync(forensicEventsPath), 'forensic events file must exist');
  const forensicRows = String(fs.readFileSync(forensicEventsPath, 'utf8') || '').split('\n').filter(Boolean);
  assert.ok(forensicRows.length >= 1, 'forensic events should be written');

  assert.ok(fs.existsSync(masterQueue), 'master queue mirror should exist');
  const mqRows = String(fs.readFileSync(masterQueue, 'utf8') || '').split('\n').filter(Boolean);
  assert.ok(mqRows.length >= 1, 'master queue should receive mirrored events');

  res = run(['evolve', `--policy=${policyPath}`]);
  assert.strictEqual(res.status, 0, `evolve should pass: ${res.stderr}`);
  assert.ok(res.payload && res.payload.ok === true, 'evolve payload should be ok');

  res = run(['status', `--policy=${policyPath}`]);
  assert.strictEqual(res.status, 0, `status should pass: ${res.stderr}`);
  assert.ok(res.payload && res.payload.ok === true, 'status payload should be ok');
  assert.ok(Number(res.payload.active_lockouts || 0) >= 1, 'status should report active lockout');

  fs.rmSync(tmp, { recursive: true, force: true });
  console.log('venom_containment_layer.test.js: OK');
} catch (err) {
  console.error(`venom_containment_layer.test.js: FAIL: ${err.message}`);
  process.exit(1);
}
