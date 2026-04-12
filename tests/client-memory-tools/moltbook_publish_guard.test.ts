#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

require(path.resolve(__dirname, '..', '..', 'client', 'runtime', 'lib', 'ts_bootstrap.ts')).installTsRequireHook();

const guard = require(path.resolve(__dirname, '..', '..', 'adapters', 'cognition', 'skills', 'moltbook', 'moltbook_publish_guard.ts'));

function captureWrites(callback) {
  const stdout = [];
  const stderr = [];
  const originalStdoutWrite = process.stdout.write.bind(process.stdout);
  const originalStderrWrite = process.stderr.write.bind(process.stderr);
  process.stdout.write = (chunk, encoding, cb) => {
    stdout.push(Buffer.isBuffer(chunk) ? chunk.toString('utf8') : String(chunk));
    if (typeof encoding === 'function') encoding();
    if (typeof cb === 'function') cb();
    return true;
  };
  process.stderr.write = (chunk, encoding, cb) => {
    stderr.push(Buffer.isBuffer(chunk) ? chunk.toString('utf8') : String(chunk));
    if (typeof encoding === 'function') encoding();
    if (typeof cb === 'function') cb();
    return true;
  };
  try {
    return { result: callback(), stdout: stdout.join(''), stderr: stderr.join('') };
  } finally {
    process.stdout.write = originalStdoutWrite;
    process.stderr.write = originalStderrWrite;
  }
}

function parseJsonOutput(text) {
  const trimmed = String(text || '').trim();
  if (!trimmed) return null;
  const lines = trimmed.split('\n');
  for (let index = lines.length - 1; index >= 0; index -= 1) {
    const candidate = lines[index].trim();
    if (!candidate.startsWith('{') || !candidate.endsWith('}')) continue;
    try {
      return JSON.parse(candidate);
    } catch {}
  }
  return null;
}

function main() {
  assert.equal(typeof guard.run, 'function');
  assert.equal(guard.lane, 'moltbook_publish_guard');

  const run = captureWrites(() => guard.run(['status']));
  assert.equal(run.result.status, 0);
  assert.equal(run.result.payload.type, 'runtime_systems_status');
  assert.equal(run.result.payload.command, 'status');
  assert.equal(run.result.routed_via, 'conduit');
  assert.equal(run.result.payload.routed_via, 'core_local');
  assert.equal(run.stderr, '');

  const cliPath = path.resolve(__dirname, '..', '..', 'adapters', 'cognition', 'skills', 'moltbook', 'moltbook_post_cli.ts');
  const cli = spawnSync(process.execPath, [cliPath, 'status'], { encoding: 'utf8' });
  assert.equal(cli.status, 0, cli.stderr || cli.stdout);
  assert.equal(cli.stderr, '');
  const payload = parseJsonOutput(cli.stdout);
  assert(payload && payload.ok === true, 'expected moltbook post cli status payload');
  assert.equal(payload.payload.payload.type, 'runtime_systems_status');
  assert.equal(payload.payload.payload.command, 'status');

  console.log(JSON.stringify({ ok: true, type: 'moltbook_publish_guard_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
