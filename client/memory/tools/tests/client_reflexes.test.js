#!/usr/bin/env node
'use strict';

const assert = require('assert');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCRIPT = path.join(ROOT, 'reflexes', 'index.js');

function run(args) {
  return spawnSync('node', [SCRIPT, ...args], {
    cwd: ROOT,
    encoding: 'utf8',
    env: { ...process.env }
  });
}

function parse(stdout) {
  try { return JSON.parse(String(stdout || '').trim()); } catch { return null; }
}

function test(name, fn) {
  try {
    fn();
    console.log(`   OK ${name}`);
  } catch (err) {
    console.error(`   FAIL ${name}: ${err && err.message ? err.message : err}`);
    process.exitCode = 1;
  }
}

console.log('client_reflexes.test.js');

test('registry exposes five reflexes with <=150 token cap', () => {
  const out = run(['list']);
  assert.strictEqual(out.status, 0, out.stderr || 'list should pass');
  const payload = parse(out.stdout);
  assert.ok(payload && payload.ok === true, 'payload should be ok');
  assert.strictEqual(Number(payload.count), 5, 'should expose 5 reflexes');
  assert.ok(Array.isArray(payload.reflexes), 'reflexes should be an array');
  for (const reflex of payload.reflexes) {
    assert.ok(Number(reflex.max_tokens_est || 0) <= 150, 'reflex token cap must be <= 150');
  }
});

test('each reflex run payload stays within declared token cap', () => {
  const listRun = run(['list']);
  const listPayload = parse(listRun.stdout);
  assert.ok(listPayload && Array.isArray(listPayload.reflexes), 'reflex list should parse');

  for (const reflex of listPayload.reflexes) {
    const execRun = run(['run', `--id=${reflex.id}`, '--input=memory matrix regression cleanup']);
    assert.strictEqual(execRun.status, 0, execRun.stderr || `run should pass for ${reflex.id}`);
    const payload = parse(execRun.stdout);
    assert.ok(payload && payload.ok === true, `payload should be ok for ${reflex.id}`);
    assert.strictEqual(payload.id, reflex.id, 'reflex id should round-trip');
    assert.ok(Number(payload.token_est || 0) <= Number(payload.max_tokens_est || 0), `token estimate must fit cap for ${reflex.id}`);
    assert.ok(Number(payload.max_tokens_est || 0) <= 150, `cap must remain <= 150 for ${reflex.id}`);
  }
});

test('unknown reflex fails closed', () => {
  const out = run(['run', '--id=not_real']);
  assert.notStrictEqual(out.status, 0, 'unknown reflex should fail');
  const payload = parse(out.stdout);
  assert.ok(payload && payload.ok === false, 'error payload should parse');
  assert.ok(String(payload.error || '').startsWith('unknown_reflex:'), 'error should indicate unknown reflex');
});

if (process.exitCode && process.exitCode !== 0) {
  console.error('client_reflexes.test.js: FAIL');
  process.exit(process.exitCode);
}

console.log('client_reflexes.test.js: OK');
