#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCRIPT = path.join(ROOT, 'systems', 'workflow', 'payment_skills_bridge.js');

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function run(args) {
  const res = spawnSync(process.execPath, [SCRIPT, ...args], {
    cwd: ROOT,
    encoding: 'utf8'
  });
  return {
    status: typeof res.status === 'number' ? res.status : 1,
    stdout: String(res.stdout || ''),
    stderr: String(res.stderr || '')
  };
}

function parseJson(stdout) {
  return JSON.parse(String(stdout || '').trim());
}

try {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'payment-bridge-'));
  const policyPath = path.join(tmp, 'config', 'payment_skills_bridge_policy.json');
  const statePath = path.join(tmp, 'state', 'workflow', 'payment_bridge', 'latest.json');
  const historyPath = path.join(tmp, 'state', 'workflow', 'payment_bridge', 'history.jsonl');
  const holdsPath = path.join(tmp, 'state', 'workflow', 'payment_bridge', 'holds.json');

  writeJson(policyPath, {
    version: '1.0-test',
    enabled: true,
    shadow_only: true,
    require_approval_note_for_live: true,
    max_single_payout_usd: 1000,
    providers: {
      stripe: { enabled: true },
      paypal: { enabled: true },
      mercury: { enabled: false }
    },
    paths: {
      state: statePath,
      history: historyPath,
      holds: holdsPath
    }
  });

  let out = run([
    'payout',
    '--policy=' + policyPath,
    '--provider=stripe',
    '--amount-usd=50',
    '--recipient=test_user',
    '--payout-id=p1',
    '--apply=1'
  ]);
  assert.strictEqual(out.status, 0, `shadow payout run should return payload: ${out.stderr}`);
  let payload = parseJson(out.stdout);
  assert.strictEqual(payload.decision, 'hold', 'shadow mode should force hold');
  assert.ok(Array.isArray(payload.blockers) && payload.blockers.includes('shadow_only_live_blocked'));
  assert.ok(fs.existsSync(holdsPath), 'hold registry should be created');

  writeJson(policyPath, {
    version: '1.0-test',
    enabled: true,
    shadow_only: false,
    require_approval_note_for_live: true,
    max_single_payout_usd: 1000,
    providers: {
      stripe: { enabled: true },
      paypal: { enabled: true },
      mercury: { enabled: false }
    },
    paths: {
      state: statePath,
      history: historyPath,
      holds: holdsPath
    }
  });

  out = run([
    'payout',
    '--policy=' + policyPath,
    '--provider=stripe',
    '--amount-usd=25',
    '--recipient=test_user',
    '--payout-id=p2',
    '--apply=1'
  ]);
  payload = parseJson(out.stdout);
  assert.strictEqual(payload.decision, 'hold', 'live payout without approval note should hold');
  assert.ok(payload.blockers.includes('missing_live_approval_note'));

  out = run([
    'release',
    '--policy=' + policyPath,
    '--payout-id=p2',
    '--approval-note=human approved'
  ]);
  payload = parseJson(out.stdout);
  assert.strictEqual(payload.decision, 'execute', 'release with approval should execute');
  assert.ok(typeof payload.reversible_token === 'string' && payload.reversible_token.length > 0, 'release should emit reversible token');

  out = run([
    'payout',
    '--policy=' + policyPath,
    '--provider=stripe',
    '--amount-usd=30',
    '--recipient=test_user',
    '--payout-id=p3',
    '--apply=1',
    '--approval-note=manual approval'
  ]);
  payload = parseJson(out.stdout);
  assert.strictEqual(payload.decision, 'execute', 'live payout with approval note should execute');
  assert.ok(payload.reversible_token, 'execute path should emit reversible token');
  assert.ok(fs.existsSync(statePath), 'state should be written');
  assert.ok(fs.existsSync(historyPath), 'history should be written');

  out = run(['status', '--policy=' + policyPath]);
  payload = parseJson(out.stdout);
  assert.strictEqual(payload.ok, true, 'status should be ok');
  assert.strictEqual(payload.available, true, 'status should expose latest state');

  console.log('payment_skills_bridge.test.js: OK');
} catch (err) {
  console.error(`payment_skills_bridge.test.js: FAIL: ${err.message}`);
  process.exit(1);
}

