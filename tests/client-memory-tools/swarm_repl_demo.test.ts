#!/usr/bin/env node
'use strict';

// SRS coverage: V6-WORKFLOW-007.6

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const DEMO = path.join(ROOT, 'client', 'runtime', 'systems', 'autonomy', 'swarm_repl_demo.ts');
const demoModule = require(DEMO);

function parseLastJson(stdout) {
  const lines = String(stdout || '').split('\n').map((row) => row.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    if (!lines[i].startsWith('{')) continue;
    try {
      return JSON.parse(lines[i]);
    } catch {}
  }
  return null;
}

function run() {
  assert.strictEqual(
    demoModule.run(['help']),
    0,
    'expected compatibility binder to preserve numeric exit codes'
  );
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'swarm-repl-demo-'));
  const state = path.join(tmpDir, 'demo-state.json');
  const result = spawnSync(process.execPath, [DEMO, 'demo', '--kind=full', `--state-path=${state}`], {
    cwd: ROOT,
    encoding: 'utf8',
  });
  assert.strictEqual(result.status, 0, result.stderr || result.stdout);
  const payload = parseLastJson(result.stdout);
  assert(payload && payload.ok === true, 'expected demo shell to emit JSON payload');
  assert.strictEqual(payload.type, 'swarm_repl_demo');
  assert.strictEqual(Boolean(payload.summary.coordinator_session_id), true);
  assert.strictEqual(Boolean(payload.summary.specialist_session_id), true);
  assert.strictEqual(Boolean(payload.summary.handoff_id), true);
  assert.strictEqual(Boolean(payload.summary.tool_manifest_id), true);
  assert.strictEqual(Boolean(payload.summary.run_id), true);
  assert.strictEqual(Boolean(payload.summary.network_id), true);
  assert.strictEqual(Boolean(payload.results.length >= 7), true);

  console.log(JSON.stringify({ ok: true, type: 'swarm_repl_demo_test' }));
}

run();
