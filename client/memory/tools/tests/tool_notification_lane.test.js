#!/usr/bin/env node
'use strict';

const fs = require('fs');
const os = require('os');
const path = require('path');
const assert = require('assert');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCRIPT = path.join(ROOT, 'systems', 'tools', 'tool_notification_lane.js');

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
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'tool-notify-lane-'));
  writeJson(path.join(tmp, 'AGENTS.md'), { ok: true });
  writeJson(path.join(tmp, 'package.json'), { name: 'tmp' });
  const policyPath = path.join(tmp, 'client', 'runtime', 'config', 'tool_notification_policy.json');
  writeJson(policyPath, {
    version: 'test',
    enabled: true,
    channels: {
      main: { enabled: true, retry_limit: 2, escalate_after: 1 }
    },
    paths: {
      latest_path: 'state/tools/notification_lane/latest.json',
      history_path: 'state/tools/notification_lane/history.jsonl',
      outbox_path: 'state/tools/notification_lane/outbox.jsonl'
    }
  });

  const out = run([
    'notify',
    `--policy=${policyPath}`,
    '--channel=main',
    '--severity=critical',
    '--topic=heartbeat',
    '--message=bridge degraded',
    '--attempt=1',
    '--apply=1'
  ], { OPENCLAW_WORKSPACE: tmp });

  assert.strictEqual(out.status, 0, out.stderr || out.stdout);
  const payload = JSON.parse(out.stdout);
  assert.strictEqual(payload.ok, true, 'notify should pass');
  assert.strictEqual(payload.escalation, true, 'critical severity must escalate');

  const outboxPath = path.join(tmp, 'client', 'runtime', 'local', 'state', 'tools', 'notification_lane', 'outbox.jsonl');
  assert.ok(fs.existsSync(outboxPath), 'outbox should be written');
  const lines = fs.readFileSync(outboxPath, 'utf8').trim().split('\n').filter(Boolean);
  assert.ok(lines.length >= 1, 'outbox should contain entries');
  console.log('tool_notification_lane.test.js: OK');
} catch (err) {
  console.error(`tool_notification_lane.test.js: FAIL: ${err.message}`);
  process.exit(1);
}
