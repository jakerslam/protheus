#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCRIPT = path.join(ROOT, 'systems', 'ops', 'source_runtime_classifier_contract.js');

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

function mkdirp(p) {
  fs.mkdirSync(p, { recursive: true });
}

try {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'source-runtime-classifier-'));
  fs.writeFileSync(path.join(tmp, 'AGENTS.md'), '# tmp\n', 'utf8');
  fs.writeFileSync(path.join(tmp, 'package.json'), '{"name":"tmp"}\n', 'utf8');

  const requiredDirs = [
    'client/local/adaptive',
    'client/local/memory',
    'client/local/logs',
    'client/local/secrets',
    'client/local/state',
    'core/local/state',
    'core/local/logs',
    'core/local/cache',
    'core/local/memory'
  ];
  for (const relDir of requiredDirs) {
    const abs = path.join(tmp, relDir);
    mkdirp(abs);
    fs.writeFileSync(path.join(abs, '.gitkeep'), '', 'utf8');
  }

  const env = { OPENCLAW_WORKSPACE: tmp };

  let out = run(['check', '--strict=1'], env);
  assert.strictEqual(out.status, 0, out.stderr);
  let payload = parseJson(out.stdout);
  assert.ok(payload && payload.ok === true, 'baseline strict check should pass');

  mkdirp(path.join(tmp, 'memory'));
  out = run(['check', '--strict=1'], env);
  assert.notStrictEqual(out.status, 0, 'legacy runtime root should fail strict check');
  payload = parseJson(out.stdout);
  assert.ok(payload && payload.checks && payload.checks.no_legacy_runtime_roots === false, 'legacy root detection expected');

  fs.rmSync(path.join(tmp, 'memory'), { recursive: true, force: true });
  fs.writeFileSync(path.join(tmp, 'client/local/state', 'rogue.ts'), 'export const rogue = true;\n', 'utf8');

  out = run(['check', '--strict=1'], env);
  assert.notStrictEqual(out.status, 0, 'runtime source file should fail strict check');
  payload = parseJson(out.stdout);
  assert.ok(payload && payload.checks && payload.checks.no_source_files_in_runtime_roots === false, 'runtime source detection expected');

  out = run(['status'], env);
  assert.strictEqual(out.status, 0, out.stderr);
  payload = parseJson(out.stdout);
  assert.ok(payload && payload.type === 'source_runtime_classifier_contract', 'status should return classifier payload');

  fs.rmSync(tmp, { recursive: true, force: true });
  console.log('source_runtime_classifier_contract.test.js: OK');
} catch (err) {
  console.error(`source_runtime_classifier_contract.test.js: FAIL: ${err && err.message ? err.message : err}`);
  process.exit(1);
}
