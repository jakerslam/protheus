#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const bridge = require('../../client/runtime/systems/autonomy/swarm_sessions_bridge.ts');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.join(ROOT, 'client', 'runtime', 'lib', 'ts_entrypoint.ts');
const OPS = path.join(ROOT, 'client', 'runtime', 'systems', 'ops', 'run_protheus_ops.ts');

function parseLastJson(stdout) {
  const text = String(stdout || '').trim();
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {}
  let depth = 0;
  let start = -1;
  let inString = false;
  let escaped = false;
  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (inString) {
      if (escaped) escaped = false;
      else if (ch === '\\') escaped = true;
      else if (ch === '"') inString = false;
      continue;
    }
    if (ch === '"') {
      inString = true;
      continue;
    }
    if (ch === '{') {
      if (depth === 0) start = i;
      depth += 1;
      continue;
    }
    if (ch === '}') {
      depth -= 1;
      if (depth === 0 && start >= 0) {
        try {
          return JSON.parse(text.slice(start, i + 1));
        } catch {}
      }
    }
  }
  const lines = text
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    if (!lines[i].startsWith('{')) continue;
    try {
      return JSON.parse(lines[i]);
    } catch {}
  }
  return null;
}

function runOps(args) {
  const out = spawnSync(process.execPath, [ENTRYPOINT, OPS].concat(args), {
    cwd: ROOT,
    encoding: 'utf8',
    env: {
      ...process.env,
      PROTHEUS_OPS_LOCAL_TIMEOUT_MS: '240000',
      INFRING_OPS_LOCAL_TIMEOUT_MS: '240000',
      PROTHEUS_OPS_USE_PREBUILT: '0',
      INFRING_OPS_USE_PREBUILT: '0',
    },
  });
  return {
    status: Number.isFinite(Number(out.status)) ? Number(out.status) : 1,
    stdout: String(out.stdout || ''),
    stderr: String(out.stderr || ''),
    payload: parseLastJson(out.stdout),
  };
}

function main() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'swarm-phase7-parity-'));
  const statePath = path.join(tmpDir, 'state.json');

  const root = bridge.sessionsSpawn({
    task: 'phase7-root',
    state_path: statePath,
  });
  const child = bridge.sessionsSpawn({
    task: 'phase7-child',
    session_id: root.session_id,
    state_path: statePath,
  });
  const handoff = bridge.sessionsHandoff({
    session_id: root.session_id,
    target_session_id: child.session_id,
    reason: 'phase7 parity handoff',
    importance: 0.6,
    context: { lane: 'phase7-rust-parity' },
    state_path: statePath,
  });
  const network = bridge.networksCreate({
    session_id: root.session_id,
    spec: {
      name: 'phase7-parity-network',
      nodes: [
        { label: 'phase7-root', role: 'planner', task: 'plan parity' },
        { label: 'phase7-child', role: 'worker', task: 'execute parity' },
      ],
      edges: [
        { from: 'phase7-root', to: 'phase7-child', relation: 'handoff', auto_handoff: true },
      ],
    },
    state_path: statePath,
  });
  assert(handoff.payload?.handoff?.handoff_id, 'expected handoff to be receipted');
  assert(network.payload?.network?.network_id, 'expected network to be created');

  const rustStatus = runOps(['swarm-runtime', 'status', `--state-path=${statePath}`, '--json']);
  assert.equal(rustStatus.status, 0, `swarm-runtime status failed\n${rustStatus.stderr}`);
  assert(rustStatus.payload, 'expected swarm runtime status payload');
  assert.equal(rustStatus.payload.ok, true);

  const state = JSON.parse(fs.readFileSync(statePath, 'utf8'));
  const sessionCount = Object.keys(state.sessions || {}).length;
  const handoffCount = Object.keys(state.handoff_registry || {}).length;
  const networkCount = Object.keys(state.network_registry || {}).length;

  assert.equal(rustStatus.payload.session_count, sessionCount);
  assert.equal(rustStatus.payload.handoff_count, handoffCount);
  assert.equal(rustStatus.payload.network_count, networkCount);
  assert.equal(rustStatus.payload.result_count >= 0, true);

  console.log(JSON.stringify({ ok: true, type: 'swarm_phase7_rust_parity_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
