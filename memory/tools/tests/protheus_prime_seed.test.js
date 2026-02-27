#!/usr/bin/env node
'use strict';

const fs = require('fs');
const os = require('os');
const path = require('path');
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

function run() {
  const repoRoot = path.resolve(__dirname, '..', '..', '..');
  const scriptPath = path.join(repoRoot, 'systems', 'ops', 'protheus_prime_seed.js');
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'protheus-prime-seed-'));

  const profilePath = path.join(tmp, 'protheus_prime_profile.json');
  writeJson(profilePath, {
    profile_id: 'protheus-prime-test',
    version: '1.0',
    mandatory_paths: [
      'systems/eye/eye_kernel.ts',
      'systems/security/guard.ts',
      'systems/ops/seed_boot_probe.ts'
    ],
    probes: {
      seed_boot_probe: true,
      startup_attestation_verify: false
    }
  });

  const manifest = runNode(repoRoot, [scriptPath, 'manifest', `--profile=${profilePath}`]);
  assert.strictEqual(manifest.status, 0, manifest.stderr || 'manifest should pass');
  const manifestPayload = parseJson(manifest.stdout);
  assert.strictEqual(manifestPayload.profile.profile_id, 'protheus-prime-test');

  const receiptPath = path.join(tmp, 'latest.json');
  const historyPath = path.join(tmp, 'history.jsonl');
  const bootstrap = runNode(repoRoot, [scriptPath, 'bootstrap', `--profile=${profilePath}`], {
    PROTHEUS_PRIME_RECEIPT_PATH: receiptPath,
    PROTHEUS_PRIME_RECEIPT_HISTORY: historyPath
  });
  assert.strictEqual(bootstrap.status, 0, bootstrap.stderr || 'bootstrap should pass with test profile');
  const bootstrapPayload = parseJson(bootstrap.stdout);
  assert.strictEqual(bootstrapPayload.ok, true, `bootstrap expected ok=true, got: ${JSON.stringify(bootstrapPayload)}`);
  assert.ok(fs.existsSync(receiptPath), 'bootstrap receipt should be written');

  console.log('protheus_prime_seed.test.js: OK');
}

try {
  run();
} catch (err) {
  console.error(`protheus_prime_seed.test.js: FAIL: ${err && err.stack ? err.stack : err.message}`);
  process.exit(1);
}
