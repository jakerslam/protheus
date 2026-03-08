#!/usr/bin/env node
'use strict';

const fs = require('fs');
const os = require('os');
const path = require('path');
const assert = require('assert');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCRIPT = path.join(ROOT, 'systems', 'tools', 'tool_context_router.js');

function writeJson(filePath, payload) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function run(args, env = {}) {
  const proc = spawnSync('node', [SCRIPT, ...args], {
    cwd: ROOT,
    encoding: 'utf8',
    env: { ...process.env, ...env }
  });
  return {
    status: Number.isFinite(Number(proc.status)) ? Number(proc.status) : 1,
    stdout: String(proc.stdout || '').trim(),
    stderr: String(proc.stderr || '').trim()
  };
}

try {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'tool-context-router-'));
  writeJson(path.join(tmp, 'AGENTS.md'), { ok: true });
  writeJson(path.join(tmp, 'package.json'), { name: 'tmp' });
  const policyPath = path.join(tmp, 'client', 'runtime', 'config', 'tool_context_router_policy.json');
  writeJson(policyPath, {
    version: 'test',
    enabled: true,
    allow_unknown_tool: false,
    score_weights: { scope: 0.5, tag_overlap: 0.3, base_priority: 0.2 },
    tool_profiles: [
      { tool: 'research', scopes: ['analysis'], tags: ['intel', 'market'], base_priority: 0.8 },
      { tool: 'assimilate', scopes: ['memory'], tags: ['memory', 'decision'], base_priority: 0.7 }
    ],
    paths: {
      latest_path: 'state/tools/tool_context_router/latest.json',
      history_path: 'state/tools/tool_context_router/history.jsonl'
    }
  });
  const context = JSON.stringify({ scope: 'analysis', tags: ['market', 'intel'], objective: 'triage' });
  const out = run(['route', `--policy=${policyPath}`, `--context-json=${context}`, '--apply=1'], {
    OPENCLAW_WORKSPACE: tmp
  });
  assert.strictEqual(out.status, 0, out.stderr || out.stdout);
  const payload = JSON.parse(out.stdout);
  assert.strictEqual(payload.ok, true, 'route should pass');
  assert.strictEqual(payload.selected_tool, 'research', 'research should win');
  const latestPath = path.join(tmp, 'client', 'runtime', 'local', 'state', 'tools', 'tool_context_router', 'latest.json');
  assert.ok(fs.existsSync(latestPath), 'latest receipt missing');
  console.log('tool_context_router.test.js: OK');
} catch (err) {
  console.error(`tool_context_router.test.js: FAIL: ${err.message}`);
  process.exit(1);
}
