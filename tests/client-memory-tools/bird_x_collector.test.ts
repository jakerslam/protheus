#!/usr/bin/env node
'use strict';

const assert = require('assert');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '../..');
const adapter = require(path.join(ROOT, 'adapters/cognition/collectors/bird_x.ts'));
const shim = require(path.join(ROOT, 'client/runtime/systems/sensory/eyes_collectors/bird_x.ts'));

async function main() {
  assert.strictEqual(typeof adapter.run, 'function');
  assert.strictEqual(typeof adapter.parseArgs, 'function');
  assert.strictEqual(typeof adapter.preflightBirdX, 'function');
  assert.strictEqual(typeof shim.run, 'function');

  const parsed = adapter.parseArgs([
    'preflight',
    '--bird-cli-present=0',
    '--query=agents',
    '--query=browser tooling',
  ]);
  assert.strictEqual(parsed.command, 'preflight');
  assert.strictEqual(parsed.birdCliPresent, false);
  assert.deepStrictEqual(parsed.queries, ['agents', 'browser tooling']);

  const preflight = await adapter.preflightBirdX({ birdCliPresent: false });
  assert.strictEqual(preflight.ok, false);
  assert.strictEqual(preflight.parser_type, 'bird_x');
  assert.ok(Array.isArray(preflight.checks));
  assert.ok(Array.isArray(preflight.failures));

  const cli = spawnSync(
    process.execPath,
    [
      path.join(ROOT, 'client/runtime/systems/sensory/eyes_collectors/bird_x.ts'),
      'preflight',
      '--bird-cli-present=0',
    ],
    {
      cwd: ROOT,
      encoding: 'utf8',
    }
  );
  const stdout = String(cli.stdout || '')
    .trim()
    .split('\n')
    .filter(Boolean)
    .at(-1);
  assert.ok(stdout, 'expected bird_x shim to emit JSON');
  const payload = JSON.parse(stdout);
  assert.strictEqual(payload.parser_type, 'bird_x');
  assert.strictEqual(payload.ok, false);
  assert.strictEqual(cli.status, 1);

  console.log(JSON.stringify({ ok: true, type: 'bird_x_collector_test', status: 'pass' }));
}

if (require.main === module) {
  main().catch((err) => {
    console.error(JSON.stringify({ ok: false, error: String(err && err.message || err) }));
    process.exit(1);
  });
}

module.exports = { main };
