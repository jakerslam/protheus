#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');

require(path.resolve(__dirname, '..', '..', 'client', 'runtime', 'lib', 'ts_bootstrap.ts')).installTsRequireHook();

const learningConduit = require(path.resolve(__dirname, '..', '..', 'client', 'runtime', 'systems', 'workflow', 'learning_conduit.ts'));

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
  try {
    return JSON.parse(trimmed);
  } catch {}
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
  assert.equal(typeof learningConduit.run, 'function');
  assert.equal(typeof learningConduit.statusCode, 'function');
  assert.equal(learningConduit.systemId, 'SYSTEMS-WORKFLOW-LEARNING_CONDUIT');
  assert.equal(learningConduit.lane, 'learning_conduit');

  const run = captureWrites(() => learningConduit.run(['status']));
  assert.equal(learningConduit.statusCode(run.result), 0);
  const payload = parseJsonOutput(run.stdout);
  assert(payload && payload.ok === true, 'expected stdout payload from learning conduit');
  assert(payload.payload && payload.payload.ok === true, 'expected conduit payload envelope');
  assert.equal(payload.payload.payload.type, 'runtime_systems_status');
  assert.equal(payload.payload.payload.system_id, 'SYSTEMS-WORKFLOW-LEARNING_CONDUIT');
  assert.equal(payload.payload.payload.command, 'status');
  assert.equal(payload.payload.payload.lane, 'runtime_systems');
  assert.equal(run.stderr, '');

  console.log(JSON.stringify({ ok: true, type: 'learning_conduit_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
