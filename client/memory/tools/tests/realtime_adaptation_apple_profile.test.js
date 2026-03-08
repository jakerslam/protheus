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
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'realtime-adapt-apple-'));
  writeJson(path.join(tmp, 'AGENTS.md'), { ok: true });
  writeJson(path.join(tmp, 'package.json'), { name: 'tmp' });

  const policyPath = path.join(tmp, 'client', 'runtime', 'config', 'realtime_adaptation_loop_policy.json');
  writeJson(policyPath, {
    version: '1.0-test',
    enabled: true,
    min_cycle_interval_ms: 1000,
    resource_ceilings: {
      max_cpu_ms: 100,
      max_tokens: 200,
      max_memory_mb: 200
    },
    profiles: {
      default: { cadence_multiplier: 1, cpu_multiplier: 1, tokens_multiplier: 1, memory_multiplier: 1 },
      low_power: { cadence_multiplier: 1, cpu_multiplier: 0.6, tokens_multiplier: 0.6, memory_multiplier: 0.7 },
      apple_silicon: { cadence_multiplier: 1, cpu_multiplier: 1.4, tokens_multiplier: 1.4, memory_multiplier: 1.2 }
    },
    platform_profile_policy: {
      enable_apple_silicon: true,
      fallback_profile_for_non_arm: 'default'
    },
    drift_gates: {
      enabled: false,
      max_drift_score: 1,
      require_covenant_ok: false
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
    '--profile=auto',
    '--platform=darwin',
    '--arch=arm64',
    '--cpu-ms=130',
    '--tokens=250',
    '--memory-mb=180'
  ], { ...envBase, PROTHEUS_NOW_ISO: '2026-03-08T10:00:00.000Z' });
  assert.strictEqual(out.status, 0, out.stderr);
  let payload = parseJson(out.stdout);
  assert.ok(payload && payload.ok === true, 'apple auto profile should allow boosted budget');
  assert.strictEqual(payload.profile, 'apple_silicon', 'auto mode should select apple profile');
  assert.strictEqual(payload.profile_source, 'hardware_auto');

  out = run([
    'cycle',
    '--profile=auto',
    '--platform=linux',
    '--arch=x64',
    '--cpu-ms=130',
    '--tokens=250',
    '--memory-mb=180'
  ], { ...envBase, PROTHEUS_NOW_ISO: '2026-03-08T10:00:02.000Z' });
  assert.strictEqual(out.status, 0, out.stderr);
  payload = parseJson(out.stdout);
  assert.ok(payload && payload.ok === false, 'fallback profile should enforce tighter default budget');
  assert.strictEqual(payload.profile, 'default', 'non-arm host should fallback to default');
  assert.ok(Array.isArray(payload.blocked_reasons) && payload.blocked_reasons.includes('cpu_ceiling_exceeded'));

  console.log('realtime_adaptation_apple_profile.test.js: OK');
} catch (err) {
  console.error(`realtime_adaptation_apple_profile.test.js: FAIL: ${err && err.message ? err.message : err}`);
  process.exit(1);
}
