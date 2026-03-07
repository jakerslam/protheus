#!/usr/bin/env node
'use strict';

const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCRIPT = path.join(ROOT, 'systems', 'ops', 'f100_a_plus_readiness_gate.js');

function run(args, env = {}) {
  return spawnSync(process.execPath, [SCRIPT, ...args], {
    cwd: ROOT,
    encoding: 'utf8',
    env: {
      ...process.env,
      ...env
    }
  });
}

function parseJson(raw) {
  try {
    return JSON.parse(String(raw || '').trim());
  } catch {
    return null;
  }
}

function fail(message) {
  console.error(`f100_a_plus_readiness_gate.test.js FAILED: ${message}`);
  process.exit(1);
}

function assert(condition, message) {
  if (!condition) {
    fail(message);
  }
}

function main() {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'f100-a-plus-gate-'));
  const statePath = path.join(tempDir, 'status.json');
  const docPath = path.join(tempDir, 'status.md');

  const relaxed = run(['run', '--strict=1', '--write=1'], {
    F100_A_PLUS_MIN_COVERAGE: '0',
    F100_A_PLUS_MIN_TAGS: '0',
    F100_A_PLUS_STATE_PATH: statePath,
    F100_A_PLUS_DOC_PATH: docPath
  });
  assert(relaxed.status === 0, `relaxed strict run should pass: ${relaxed.stderr}`);
  const relaxedPayload = parseJson(relaxed.stdout);
  assert(relaxedPayload && relaxedPayload.ok === true, 'relaxed payload should be ok');
  assert(fs.existsSync(statePath), 'state path should exist');
  assert(fs.existsSync(docPath), 'doc path should exist');

  const forcedFail = run(['run', '--strict=1', '--write=0'], {
    F100_A_PLUS_MIN_COVERAGE: '101',
    F100_A_PLUS_MIN_TAGS: '999',
    F100_A_PLUS_STATE_PATH: statePath
  });
  assert(forcedFail.status !== 0, 'forced strict run should fail');

  const status = run(['status'], {
    F100_A_PLUS_STATE_PATH: statePath
  });
  assert(status.status === 0, 'status should read state path');
  const statusPayload = parseJson(status.stdout);
  assert(statusPayload && typeof statusPayload.ok === 'boolean', 'status payload should include ok field');

  console.log('f100_a_plus_readiness_gate.test.js: OK');
}

main();
