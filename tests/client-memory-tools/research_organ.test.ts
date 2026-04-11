#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');

const researchOrgan = require('../../client/runtime/systems/research/research_organ.ts');

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
  assert.equal(typeof researchOrgan.run, 'function');
  assert.equal(typeof researchOrgan.statusCode, 'function');
  assert.equal(researchOrgan.systemId, 'SYSTEMS-RESEARCH-RESEARCH_ORGAN');
  assert.equal(researchOrgan.lane, 'research_organ');

  const run = captureWrites(() => researchOrgan.run(['status']));
  assert.equal(researchOrgan.statusCode(run.result), 0);
  const payload = parseJsonOutput(run.stdout);
  assert(payload && payload.ok === true, 'expected stdout payload from research organ');
  assert(payload.payload && payload.payload.ok === true, 'expected conduit payload envelope');
  assert.equal(payload.payload.payload.type, 'runtime_systems_status');
  assert.equal(payload.payload.payload.system_id, 'SYSTEMS-RESEARCH-RESEARCH_ORGAN');
  assert.equal(payload.payload.payload.command, 'status');
  assert.equal(payload.payload.payload.lane, 'runtime_systems');
  assert.equal(run.stderr, '');

  console.log(JSON.stringify({ ok: true, type: 'research_organ_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
