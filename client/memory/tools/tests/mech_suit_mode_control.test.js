#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const ENTRY = path.join(ROOT, 'lib', 'ts_entrypoint.js');
const SCRIPT = path.join(ROOT, 'systems', 'ops', 'mech_suit_mode_control.ts');

function parseJson(text) {
  const raw = String(text || '').trim();
  if (!raw) return null;
  try { return JSON.parse(raw); } catch {}
  const lines = raw.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try { return JSON.parse(lines[i]); } catch {}
  }
  return null;
}

function run(args, env = {}) {
  const out = spawnSync(process.execPath, [ENTRY, SCRIPT, ...args], {
    cwd: ROOT,
    encoding: 'utf8',
    env: {
      ...process.env,
      ...env
    }
  });
  return {
    status: Number.isFinite(out.status) ? Number(out.status) : 1,
    stdout: String(out.stdout || ''),
    stderr: String(out.stderr || ''),
    payload: parseJson(String(out.stdout || ''))
  };
}

try {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'mech-suit-control-'));
  const policyPath = path.join(tempRoot, 'mech_suit_mode_policy.json');
  fs.writeFileSync(policyPath, `${JSON.stringify({ version: '1.0', enabled: true }, null, 2)}\n`, 'utf8');
  const env = { OPENCLAW_WORKSPACE: tempRoot, MECH_SUIT_MODE_POLICY_PATH: policyPath };

  let out = run(['status'], env);
  assert.strictEqual(out.status, 0, out.stderr || out.stdout);
  assert.ok(out.payload && out.payload.ok === true, 'status should succeed');
  assert.strictEqual(out.payload.ambient_mode_active, true, 'initial mode should be on');

  out = run(['off'], env);
  assert.strictEqual(out.status, 0, out.stderr || out.stdout);
  assert.ok(out.payload && out.payload.ok === true, 'off should succeed');
  assert.strictEqual(out.payload.ambient_mode_active, false, 'mode should be off');

  out = run(['set', '--enabled=1'], env);
  assert.strictEqual(out.status, 0, out.stderr || out.stdout);
  assert.ok(out.payload && out.payload.ok === true, 'set enabled should succeed');
  assert.strictEqual(out.payload.ambient_mode_active, true, 'mode should be on after set');

  const raw = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
  assert.strictEqual(raw.enabled, true, 'policy file should persist enabled=true');

  fs.rmSync(tempRoot, { recursive: true, force: true });
  console.log('mech_suit_mode_control.test.js: OK');
} catch (err) {
  console.error(`mech_suit_mode_control.test.js: FAIL: ${err.message}`);
  process.exit(1);
}

