#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = process.cwd();
const ENTRY = path.join(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const SCRIPT = path.join(ROOT, 'tests/tooling/scripts/ci/lane_command_dispatch.ts');

function makeTempRegistry() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'lane-dispatch-'));
  const registryPath = path.join(dir, 'lane_command_registry.json');
  fs.writeFileSync(
    registryPath,
    JSON.stringify(
      {
        version: 'test',
        run: {
          'VTEST-001': {
            id: 'VTEST-001',
            command: 'node -e "process.stdout.write(\'lane-ok\')"',
            source_script: 'lane:vtest-001:run',
          },
        },
        test: {
          'VTEST-001': {
            id: 'VTEST-001',
            command: 'node -e "process.stdout.write(\'test-ok\')"',
            source_script: 'test:lane:vtest-001',
          },
        },
      },
      null,
      2,
    ),
  );
  return registryPath;
}

function run(args) {
  return spawnSync('node', [ENTRY, SCRIPT, ...args], {
    cwd: ROOT,
    encoding: 'utf8',
  });
}

const registryPath = makeTempRegistry();

{
  const child = run(['list', `--registry=${registryPath}`, '--mode=all', '--json=1']);
  assert.equal(child.status, 0);
  const payload = JSON.parse(child.stdout);
  assert.equal(payload.ok, true);
  assert.equal(payload.count, 2);
}

{
  const child = run(['run', `--registry=${registryPath}`, '--id=VTEST-001']);
  assert.equal(child.status, 0);
}

{
  const child = run(['test', `--registry=${registryPath}`, '--id=VTEST-001']);
  assert.equal(child.status, 0);
}

{
  const child = run(['run', `--registry=${registryPath}`, '--id=VTEST-404']);
  assert.notEqual(child.status, 0);
}

console.log('ok lane_command_dispatch');
