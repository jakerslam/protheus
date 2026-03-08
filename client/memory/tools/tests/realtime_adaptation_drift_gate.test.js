#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCRIPT = path.join(ROOT, 'systems', 'adaptive', 'realtime_adaptation_loop.js');

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
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'realtime-adapt-drift-'));
  writeJson(path.join(tmp, 'AGENTS.md'), { ok: true });
  writeJson(path.join(tmp, 'package.json'), { name: 'tmp' });

  const policyPath = path.join(tmp, 'client', 'config', 'realtime_adaptation_loop_policy.json');
  writeJson(policyPath, {
    version: '1.0-test',
    enabled: true,
    min_cycle_interval_ms: 1000,
    resource_ceilings: {
      max_cpu_ms: 250,
      max_tokens: 400,
      max_memory_mb: 256
    },
    drift_gates: {
      enabled: true,
      max_drift_score: 0.4,
      require_covenant_ok: true
    },
    paths: {
      state_path: path.join(tmp, 'state', 'adaptive', 'realtime_adaptation_loop', 'state.json'),
      latest_path: path.join(tmp, 'state', 'adaptive', 'realtime_adaptation_loop', 'latest.json'),
      receipts_path: path.join(tmp, 'state', 'adaptive', 'realtime_adaptation_loop', 'receipts.jsonl')
    }
  });

  const envBase = {
    OPENCLAW_WORKSPACE: tmp,
    REALTIME_ADAPTATION_LOOP_POLICY_PATH: policyPath
  };

  let out = run([
    'cycle',
    '--trigger=interaction',
    '--cpu-ms=90',
    '--tokens=120',
    '--memory-mb=128',
    '--drift-score=0.75',
    '--covenant-ok=1'
  ], { ...envBase, PROTHEUS_NOW_ISO: '2026-03-08T09:00:00.000Z' });
  assert.strictEqual(out.status, 0, out.stderr);
  let payload = parseJson(out.stdout);
  assert.ok(payload && payload.ok === false, 'drift score above threshold should block');
  assert.ok(Array.isArray(payload.blocked_reasons) && payload.blocked_reasons.includes('drift_score_exceeded'));

  out = run([
    'cycle',
    '--trigger=interaction',
    '--cpu-ms=90',
    '--tokens=120',
    '--memory-mb=128',
    '--drift-score=0.2',
    '--covenant-ok=0'
  ], { ...envBase, PROTHEUS_NOW_ISO: '2026-03-08T09:00:03.000Z' });
  assert.strictEqual(out.status, 0, out.stderr);
  payload = parseJson(out.stdout);
  assert.ok(payload && payload.ok === false, 'covenant denial should block');
  assert.ok(Array.isArray(payload.blocked_reasons) && payload.blocked_reasons.includes('covenant_gate_denied'));

  out = run([
    'cycle',
    '--trigger=interaction',
    '--cpu-ms=90',
    '--tokens=120',
    '--memory-mb=128',
    '--drift-score=0.2',
    '--covenant-ok=1'
  ], { ...envBase, PROTHEUS_NOW_ISO: '2026-03-08T09:00:06.000Z' });
  assert.strictEqual(out.status, 0, out.stderr);
  payload = parseJson(out.stdout);
  assert.ok(payload && payload.ok === true, 'within drift/covenant bounds should pass');
  console.log('realtime_adaptation_drift_gate.test.js: OK');
} catch (err) {
  console.error(`realtime_adaptation_drift_gate.test.js: FAIL: ${err && err.message ? err.message : err}`);
  process.exit(1);
}
