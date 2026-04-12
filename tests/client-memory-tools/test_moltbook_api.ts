#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');

require(path.resolve(__dirname, '..', '..', 'client', 'runtime', 'lib', 'ts_bootstrap.ts')).installTsRequireHook();

const api = require(path.resolve(__dirname, '..', '..', 'adapters', 'cognition', 'skills', 'moltbook', 'moltbook_api.ts'));

function withEnv(name, value, callback) {
  const prior = process.env[name];
  if (value == null) delete process.env[name];
  else process.env[name] = value;
  try {
    return callback();
  } finally {
    if (prior == null) delete process.env[name];
    else process.env[name] = prior;
  }
}

function main() {
  assert.equal(typeof api.canonicalizeApiBase, 'function');
  assert.equal(typeof api.resolveApiBases, 'function');
  assert.deepEqual(api.DEFAULT_API_BASES, [
    'https://www.moltbook.com/api/v1',
    'https://api.moltbook.com/api/v1'
  ]);

  assert.equal(
    api.canonicalizeApiBase('https://api.moltbook.com'),
    'https://api.moltbook.com/api/v1'
  );
  assert.equal(
    api.canonicalizeApiBase('https://www.moltbook.com/api/v1/posts?sort=hot'),
    'https://www.moltbook.com/api/v1'
  );
  assert.equal(api.canonicalizeApiBase('http://api.moltbook.com/api/v1'), null);
  assert.equal(api.canonicalizeApiBase('https://evil.example/api/v1'), null);
  assert.equal(api.canonicalizeApiBase('https://api.moltbook.com/custom'), null);

  withEnv(
    'MOLTBOOK_API_BASES',
    'https://evil.example/api/v1, https://api.moltbook.com/api/v1/posts, https://api.moltbook.com',
    () => {
      assert.deepEqual(api.resolveApiBases(), ['https://api.moltbook.com/api/v1']);
    }
  );

  withEnv('MOLTBOOK_API_BASES', 'https://evil.example/api/v1', () => {
    assert.deepEqual(api.resolveApiBases(), api.DEFAULT_API_BASES);
  });

  console.log(JSON.stringify({ ok: true, type: 'moltbook_api_test' }));
}

try {
  main();
} catch (err) {
  console.error(err && err.stack ? err.stack : String(err));
  process.exit(1);
}
