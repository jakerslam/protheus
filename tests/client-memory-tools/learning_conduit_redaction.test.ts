#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.join(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const CLIENT_WRAPPER = path.join(ROOT, 'client/runtime/systems/workflow/learning_conduit.ts');

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
  const secret = 'abc123secret-token';
  const proc = spawnSync(process.execPath, [ENTRYPOINT, CLIENT_WRAPPER, 'run', `--token=${secret}`], {
    cwd: ROOT,
    encoding: 'utf8',
  });
  assert.equal(proc.status, 0, proc.stderr || proc.stdout || 'learning conduit run failed');
  assert.equal(
    String(proc.stderr || '').includes('ERR_INVALID_ARG_TYPE'),
    false,
    `unexpected exit-code bug resurfaced: ${proc.stderr}`,
  );
  assert.equal(String(proc.stdout || '').includes(secret), false, 'stdout should not echo sensitive passthrough values');
  assert.equal(String(proc.stderr || '').includes(secret), false, 'stderr should not echo sensitive passthrough values');

  const payload = parseJsonOutput(proc.stdout);
  assert(payload && payload.ok === true, 'expected outer payload envelope');
  assert(payload.payload && payload.payload.ok === true, 'expected conduit payload envelope');
  assert.equal(payload.payload.payload.type, 'runtime_systems_run');
  assert.equal(payload.payload.payload.system_id, 'SYSTEMS-WORKFLOW-LEARNING_CONDUIT');

  console.log(JSON.stringify({ ok: true, type: 'learning_conduit_redaction_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
