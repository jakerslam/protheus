#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCRIPT = path.join(ROOT, 'systems', 'ops', 'migrate_to_planes.js');

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
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'migrate-to-planes-'));
  fs.writeFileSync(path.join(tmp, 'AGENTS.md'), '# tmp\n', 'utf8');
  fs.writeFileSync(path.join(tmp, 'package.json'), '{"name":"tmp"}\n', 'utf8');
  fs.mkdirSync(path.join(tmp, '.git'), { recursive: true });

  fs.mkdirSync(path.join(tmp, 'state', 'ops'), { recursive: true });
  fs.writeFileSync(path.join(tmp, 'state', 'ops', 'latest.json'), '{"ok":true}\n', 'utf8');
  fs.mkdirSync(path.join(tmp, '.private-lenses'), { recursive: true });
  fs.writeFileSync(path.join(tmp, '.private-lenses', 'README.md'), 'private\n', 'utf8');
  fs.mkdirSync(path.join(tmp, 'client', 'logs'), { recursive: true });
  fs.writeFileSync(path.join(tmp, 'client', 'logs', 'session.log'), 'line\n', 'utf8');
  fs.mkdirSync(path.join(tmp, 'core', 'local'), { recursive: true });

  const env = { OPENCLAW_WORKSPACE: tmp };

  let out = run(['plan'], env);
  assert.strictEqual(out.status, 0, out.stderr);
  let payload = parseJson(out.stdout);
  assert.ok(payload && payload.ok === true, 'plan should succeed');
  assert.ok(Array.isArray(payload.rows) && payload.rows.length > 0, 'plan should include mappings');

  out = run(['run', '--apply=1', '--move-untracked=1', '--compat-symlinks=0'], env);
  assert.strictEqual(out.status, 0, out.stderr);
  payload = parseJson(out.stdout);
  assert.ok(payload && payload.ok === true, 'run apply should succeed');
  assert.ok(payload.migrated >= 1, 'at least one mapping should migrate');
  assert.ok(payload.migration_id, 'run should emit migration id');

  assert.ok(fs.existsSync(path.join(tmp, 'client', 'runtime', 'local', 'state', 'ops', 'latest.json')), 'state should migrate into client/runtime/local/state');
  assert.ok(fs.existsSync(path.join(tmp, 'client', 'runtime', 'local', 'private-lenses', 'README.md')), 'private lenses should mirror into client/runtime/local/private-lenses');
  assert.ok(fs.existsSync(path.join(tmp, 'client', 'runtime', 'local', 'logs', 'session.log')), 'client logs should mirror into client/runtime/local/logs');
  const statePath = path.join(tmp, 'state');
  assert.strictEqual(fs.existsSync(statePath), false, 'state root should be removed after direct migration');

  out = run(['rollback', '--id=latest'], env);
  assert.strictEqual(out.status, 0, out.stderr);
  payload = parseJson(out.stdout);
  assert.ok(payload && payload.ok === true, 'rollback should succeed');
  assert.ok(fs.existsSync(path.join(tmp, 'state', 'ops', 'latest.json')), 'state file should restore at root on rollback');
  assert.strictEqual(fs.lstatSync(path.join(tmp, 'state')).isSymbolicLink(), false, 'state symlink should be removed by rollback');

  out = run(['status'], env);
  assert.strictEqual(out.status, 0, out.stderr);
  payload = parseJson(out.stdout);
  assert.ok(payload && payload.ok === true, 'status should return latest receipt');

  fs.rmSync(tmp, { recursive: true, force: true });
  console.log('migrate_to_planes.test.js: OK');
} catch (err) {
  console.error(`migrate_to_planes.test.js: FAIL: ${err && err.message ? err.message : err}`);
  process.exit(1);
}
