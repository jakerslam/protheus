#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = process.cwd();
const ENTRY = path.join(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const SCRIPT = path.join(ROOT, 'tests/tooling/scripts/ci/tooling_registry_runner.ts');

function run(args) {
  return spawnSync('node', [ENTRY, SCRIPT, ...args], {
    cwd: ROOT,
    encoding: 'utf8',
  });
}

const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'tooling-runner-'));
const listOut = path.join(tmpDir, 'list.json');
const gateOut = path.join(tmpDir, 'gate.json');
const profileOut = path.join(tmpDir, 'profile.json');

{
  const child = run(['list', '--json=1', `--out=${listOut}`]);
  assert.equal(child.status, 0);
  assert.equal(fs.existsSync(listOut), true);
  const payload = JSON.parse(fs.readFileSync(listOut, 'utf8'));
  assert.equal(payload.ok, true);
  assert(payload.gates.some((row) => row.id === 'ops:arch:conformance'));
  assert(payload.profiles.some((row) => row.id === 'fast'));
}

{
  const child = run(['gate', '--id=ops:arch:conformance', '--strict=1', `--out=${gateOut}`]);
  assert.equal(child.status, 0);
  const payload = JSON.parse(child.stdout);
  assert.equal(payload.ok, true);
  assert.equal(payload.gate_id, 'ops:arch:conformance');
  assert.equal(fs.existsSync(gateOut), true);
}

{
  const child = run(['profile', '--id=fast', '--strict=1', `--out=${profileOut}`]);
  assert.equal(child.status, 0);
  const payload = JSON.parse(child.stdout);
  assert.equal(payload.ok, true);
  assert.equal(payload.profile_id, 'fast');
  assert.equal(fs.existsSync(profileOut), true);
}

console.log(JSON.stringify({ ok: true, type: 'tooling_registry_runner_test' }));
