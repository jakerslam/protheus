#!/usr/bin/env node
'use strict';

const fs = require('fs');
const os = require('os');
const path = require('path');
const assert = require('assert');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCRIPT = path.join(ROOT, 'systems', 'tools', 'shadow_tool_notification_bridge.js');

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
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'shadow-tool-bridge-'));
  writeJson(path.join(tmp, 'AGENTS.md'), { ok: true });
  writeJson(path.join(tmp, 'package.json'), { name: 'tmp' });

  const bridgePolicy = path.join(tmp, 'client', 'runtime', 'config', 'shadow_tool_notification_bridge_policy.json');
  const routerPolicy = path.join(tmp, 'client', 'runtime', 'config', 'tool_context_router_policy.json');
  const notifyPolicy = path.join(tmp, 'client', 'runtime', 'config', 'tool_notification_policy.json');

  writeJson(bridgePolicy, {
    version: 'test',
    enabled: true,
    allowlist: { shadows: ['alpha'], personas: ['vikram'] },
    notification: { default_channel: 'main', default_severity: 'info' },
    paths: {
      latest_path: 'state/tools/shadow_tool_bridge/latest.json',
      history_path: 'state/tools/shadow_tool_bridge/history.jsonl'
    }
  });
  writeJson(routerPolicy, {
    version: 'test',
    enabled: true,
    allow_unknown_tool: false,
    score_weights: { scope: 0.5, tag_overlap: 0.3, base_priority: 0.2 },
    tool_profiles: [
      { tool: 'assimilate', scopes: ['memory'], tags: ['memory', 'decision'], base_priority: 0.9 }
    ],
    paths: {
      latest_path: 'state/tools/tool_context_router/latest.json',
      history_path: 'state/tools/tool_context_router/history.jsonl'
    }
  });
  writeJson(notifyPolicy, {
    version: 'test',
    enabled: true,
    channels: { main: { enabled: true, retry_limit: 1, escalate_after: 0 } },
    paths: {
      latest_path: 'state/tools/notification_lane/latest.json',
      history_path: 'state/tools/notification_lane/history.jsonl',
      outbox_path: 'state/tools/notification_lane/outbox.jsonl'
    }
  });

  const context = JSON.stringify({ scope: 'memory', tags: ['decision', 'memory'] });
  const out = run([
    'bridge',
    `--policy=${bridgePolicy}`,
    `--router-policy=${routerPolicy}`,
    `--notify-policy=${notifyPolicy}`,
    '--shadow=alpha',
    '--persona=vikram',
    `--context-json=${context}`,
    '--message=test bridge notification',
    '--apply=1'
  ], { OPENCLAW_WORKSPACE: tmp });

  assert.strictEqual(out.status, 0, out.stderr || out.stdout);
  const payload = JSON.parse(out.stdout);
  assert.strictEqual(payload.ok, true, 'bridge should pass');
  assert.strictEqual(payload.selected_tool, 'assimilate', 'bridge must route expected tool');

  const outboxPath = path.join(tmp, 'client', 'runtime', 'local', 'state', 'tools', 'notification_lane', 'outbox.jsonl');
  assert.ok(fs.existsSync(outboxPath), 'bridge should emit notification');
  console.log('shadow_tool_notification_bridge.test.js: OK');
} catch (err) {
  console.error(`shadow_tool_notification_bridge.test.js: FAIL: ${err.message}`);
  process.exit(1);
}
