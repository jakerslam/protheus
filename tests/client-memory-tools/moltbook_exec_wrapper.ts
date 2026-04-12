#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

require(path.resolve(__dirname, '..', '..', 'client', 'runtime', 'lib', 'ts_bootstrap.ts')).installTsRequireHook();

const runtimeApi = require(path.resolve(__dirname, '..', '..', 'client', 'runtime', 'lib', 'moltbook_api.ts'));

async function withEnv(name, value, callback) {
  const prior = process.env[name];
  if (value == null) delete process.env[name];
  else process.env[name] = value;
  try {
    return await callback();
  } finally {
    if (prior == null) delete process.env[name];
    else process.env[name] = prior;
  }
}

function parseJsonOutput(text) {
  const lines = String(text || '').trim().split('\n');
  for (let index = lines.length - 1; index >= 0; index -= 1) {
    const candidate = lines[index].trim();
    if (!candidate.startsWith('{') || !candidate.endsWith('}')) continue;
    try {
      return JSON.parse(candidate);
    } catch {}
  }
  return null;
}

function runStatus(relPath) {
  const fullPath = path.resolve(__dirname, '..', '..', relPath);
  const result = spawnSync(process.execPath, [fullPath, 'status'], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  const payload = parseJsonOutput(result.stdout);
  assert(payload && payload.ok === true, `expected status payload for ${relPath}`);
  return payload;
}

async function main() {
  assert.equal(typeof runtimeApi.moltbook_getHotPosts, 'function');
  assert.equal(typeof runtimeApi.resolveApiBases, 'function');

  const fixturePath = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'moltbook-fixture-')), 'posts.json');
  fs.writeFileSync(
    fixturePath,
    JSON.stringify({
      posts: [
        { id: 'p1', title: 'One' },
        { id: 'p2', title: 'Two' },
        { id: 'p3', title: 'Three' }
      ]
    }),
    'utf8'
  );
  const posts = await withEnv('MOLTBOOK_HOT_POSTS_FIXTURE', fixturePath, async () =>
    runtimeApi.moltbook_getHotPosts(2)
  );
  assert.equal(Array.isArray(posts), true);
  assert.equal(posts.length, 2);
  assert.equal(posts[0].id, 'p1');
  assert.equal(posts[1].id, 'p2');

  const wrapperPaths = [
    'adapters/cognition/skills/moltbook/actuation_adapter.ts',
    'adapters/cognition/skills/moltbook/moltbook_publish_guard.ts',
    'adapters/cognition/skills/moltbook/proposal_template.ts'
  ];
  for (const relPath of wrapperPaths) {
    const payload = runStatus(relPath);
    assert.equal(payload.payload.payload.type, 'runtime_systems_status');
    assert.equal(payload.payload.payload.command, 'status');
  }

  console.log(JSON.stringify({ ok: true, type: 'moltbook_exec_wrapper_test' }));
}

if (require.main === module) {
  main().catch((err) => {
    console.error(err && err.stack ? err.stack : String(err));
    process.exit(1);
  });
}

module.exports = { main };
