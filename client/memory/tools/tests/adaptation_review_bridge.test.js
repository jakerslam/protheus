#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCRIPT = path.join(ROOT, 'systems', 'adaptive', 'adaptation_review_bridge.js');

function writeJson(filePath, payload) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function run(args, env = {}) {
  const proc = spawnSync('node', [SCRIPT, ...args], {
    cwd: ROOT,
    encoding: 'utf8',
    env: { ...process.env, ...env }
  });
  return {
    status: Number.isFinite(Number(proc.status)) ? Number(proc.status) : 1,
    stdout: String(proc.stdout || ''),
    stderr: String(proc.stderr || '')
  };
}

function parseJson(stdout) {
  const txt = String(stdout || '').trim();
  return txt ? JSON.parse(txt) : null;
}

try {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'adapt-review-bridge-'));
  writeJson(path.join(tmp, 'AGENTS.md'), { ok: true });
  writeJson(path.join(tmp, 'package.json'), { name: 'tmp' });
  const policyPath = path.join(tmp, 'client', 'config', 'adaptation_review_bridge_policy.json');
  writeJson(policyPath, {
    version: 'test',
    enabled: true,
    high_impact_threshold: 0.7,
    require_shadow_persona_for_high_impact: true,
    paths: {
      latest_path: 'state/adaptive/review_bridge/latest.json',
      history_path: 'state/adaptive/review_bridge/history.jsonl',
      queue_path: 'state/adaptive/review_bridge/review_queue.jsonl'
    }
  });

  let out = run([
    'submit',
    `--policy=${policyPath}`,
    '--risk-score=0.8',
    '--adaptation-receipt-id=abc123',
    '--summary=high impact change',
    '--apply=1'
  ], { OPENCLAW_WORKSPACE: tmp });
  assert.notStrictEqual(out.status, 0, 'high impact submission without shadow/persona should fail');

  out = run([
    'submit',
    `--policy=${policyPath}`,
    '--risk-score=0.8',
    '--adaptation-receipt-id=abc123',
    '--shadow=alpha',
    '--persona=vikram',
    '--summary=high impact change',
    '--apply=1'
  ], { OPENCLAW_WORKSPACE: tmp });
  assert.strictEqual(out.status, 0, out.stderr);
  let payload = parseJson(out.stdout);
  assert.ok(payload && payload.ok === true, 'high impact submission should pass with review metadata');
  assert.strictEqual(payload.review_status, 'pending_review');

  out = run([
    'submit',
    `--policy=${policyPath}`,
    '--risk-score=0.2',
    '--adaptation-receipt-id=xyz789',
    '--summary=low impact update',
    '--apply=1'
  ], { OPENCLAW_WORKSPACE: tmp });
  assert.strictEqual(out.status, 0, out.stderr);
  payload = parseJson(out.stdout);
  assert.strictEqual(payload.review_status, 'auto_approved');
  console.log('adaptation_review_bridge.test.js: OK');
} catch (err) {
  console.error(`adaptation_review_bridge.test.js: FAIL: ${err && err.message ? err.message : err}`);
  process.exit(1);
}
