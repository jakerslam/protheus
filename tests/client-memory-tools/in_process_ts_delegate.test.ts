#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const { invokeTsModuleSync, invokeTsModuleAsync } = require('../../client/runtime/lib/in_process_ts_delegate.ts');

function writeFixture(dir, name, source) {
  const filePath = path.join(dir, name);
  fs.writeFileSync(filePath, source, 'utf8');
  return filePath;
}

async function main() {
  const previousCwd = process.cwd();
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'ts-delegate-'));
  const realTmpDir = fs.realpathSync(tmpDir);

  const syncPath = writeFixture(
    tmpDir,
    'sync_fixture.ts',
    "module.exports = { run(argv = []) { console.log(JSON.stringify({ kind: 'sync', cwd: process.cwd(), argv })); return 7; } };",
  );
  const syncOut = invokeTsModuleSync(syncPath, {
    argv: ['alpha', 'beta'],
    cwd: tmpDir,
    exportName: 'run',
  });
  const syncPayload = JSON.parse(String(syncOut.stdout || '').trim());
  assert.equal(syncOut.status, 7);
  assert.equal(syncPayload.kind, 'sync');
  assert.equal(fs.realpathSync(syncPayload.cwd), realTmpDir);
  assert.deepEqual(syncPayload.argv, ['alpha', 'beta']);
  assert.equal(process.cwd(), previousCwd);

  const asyncPath = writeFixture(
    tmpDir,
    'async_fixture.ts',
    "module.exports = { async run(argv = []) { console.log(JSON.stringify({ kind: 'async', cwd: process.cwd(), argv })); return 3; } };",
  );
  const asyncOut = await invokeTsModuleAsync(asyncPath, {
    argv: ['gamma'],
    cwd: tmpDir,
    exportName: 'run',
  });
  const asyncPayload = JSON.parse(String(asyncOut.stdout || '').trim());
  assert.equal(asyncOut.status, 3);
  assert.equal(asyncPayload.kind, 'async');
  assert.equal(fs.realpathSync(asyncPayload.cwd), realTmpDir);
  assert.deepEqual(asyncPayload.argv, ['gamma']);
  assert.equal(process.cwd(), previousCwd);

  fs.rmSync(tmpDir, { recursive: true, force: true });
  console.log(JSON.stringify({ ok: true, type: 'in_process_ts_delegate_test' }));
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
