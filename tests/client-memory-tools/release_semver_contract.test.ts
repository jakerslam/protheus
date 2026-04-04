#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const path = require('node:path');
const { execFileSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');

function runPlan(args = []) {
  const out = execFileSync(
    process.execPath,
    [
      'client/runtime/lib/ts_entrypoint.ts',
      'client/runtime/systems/ops/release_semver_contract.ts',
    ].concat(args),
    {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    }
  );
  return JSON.parse(String(out || '{}'));
}

function main() {
  const payload = runPlan(['status', '--strict=1', '--write=0', '--pretty=0']);
  assert.strictEqual(payload.ok, true, 'plan should be ok');
  assert.strictEqual(typeof payload.release_ready, 'boolean');
  assert.strictEqual(typeof payload.next_version, 'string');
  assert.strictEqual(typeof payload.next_tag, 'string');
  assert.strictEqual(typeof payload.current_version, 'string');
  assert.strictEqual(typeof payload.bump, 'string');
  assert.ok(['major', 'minor', 'patch', 'none'].includes(payload.bump), 'bump classification should be valid');
  process.stdout.write(
    `${JSON.stringify(
      {
        ok: true,
        checked: 'release_semver_contract',
        release_ready: payload.release_ready,
        bump: payload.bump,
        next_tag: payload.next_tag,
      },
      null,
      2
    )}\n`
  );
}

if (require.main === module) {
  main();
}
