#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

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
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'thorn-swarm-protocol-'));
  const statePath = path.join(tmpDir, 'swarm-state.json');

  const spawn = runOps(['swarm-runtime', 'spawn', '--task=thorn-target', `--state-path=${statePath}`, '--json']);
  assert.equal(spawn.status, 0, `swarm-runtime spawn failed\n${spawn.stderr}`);
  assert(spawn.payload, 'expected spawn payload');
  const sessionId = spawn.payload.payload && spawn.payload.payload.session_id;
  assert(sessionId, 'expected spawned thorn target session id');

  const quarantine = runOps([
    'swarm-runtime',
    'thorn',
    'quarantine',
    `--session-id=${sessionId}`,
    '--anomaly-type=exfil',
    '--reason=test_quarantine',
    `--state-path=${statePath}`,
    '--json',
  ]);
  assert.equal(quarantine.status, 0, `thorn quarantine failed\n${quarantine.stderr}`);
  assert.equal(quarantine.payload.type, 'swarm_runtime_thorn_quarantine');

  const stateAfterQuarantine = JSON.parse(fs.readFileSync(statePath, 'utf8'));
  assert.equal(stateAfterQuarantine.sessions[sessionId].status, 'quarantined_thorn');

  const release = runOps([
    'swarm-runtime',
    'thorn',
    'release',
    `--session-id=${sessionId}`,
    '--reason=threat_removed',
    `--state-path=${statePath}`,
    '--json',
  ]);
  assert.equal(release.status, 0, `thorn release failed\n${release.stderr}`);
  assert.equal(release.payload.type, 'swarm_runtime_thorn_release');

  const stateAfterRelease = JSON.parse(fs.readFileSync(statePath, 'utf8'));
  assert.equal(stateAfterRelease.sessions[sessionId].reachable, true);
  assert(
    Object.values(stateAfterRelease.sessions).some(
      (row) => row && row.thorn_cell === true && row.status === 'thorn_destroyed'
    ),
    'expected thorn sacrificial cell to self-destruct after release'
  );

  const securityStatus = runOps(['security-plane', 'thorn-swarm-protocol', 'status', '--json']);
  assert.equal(securityStatus.status, 0, `security-plane thorn status failed\n${securityStatus.stderr}`);
  assert.equal(securityStatus.payload.type, 'swarm_runtime_thorn_status');

  console.log(JSON.stringify({ ok: true, type: 'thorn_swarm_protocol_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
