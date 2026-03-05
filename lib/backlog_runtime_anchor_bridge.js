'use strict';

const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..');

function parseJsonPayload(raw) {
  const text = String(raw == null ? '' : raw).trim();
  if (!text) return null;
  try { return JSON.parse(text); } catch {}
  const lines = text.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try { return JSON.parse(lines[i]); } catch {}
  }
  return null;
}

function runRustAnchor(laneId) {
  const args = [
    'run',
    '--quiet',
    '--manifest-path',
    'crates/ops/Cargo.toml',
    '--bin',
    'protheus-ops',
    '--',
    'backlog-runtime-anchor',
    'build',
    `--lane-id=${laneId}`
  ];
  const out = spawnSync('cargo', args, {
    cwd: ROOT,
    encoding: 'utf8',
    maxBuffer: 10 * 1024 * 1024,
    env: {
      ...process.env,
      PROTHEUS_NODE_BINARY: process.execPath || 'node'
    }
  });

  const payload = parseJsonPayload(out.stdout);
  if (Number(out.status) === 0 && payload && payload.ok === true) {
    return payload;
  }

  return {
    ok: false,
    type: 'backlog_runtime_anchor_bridge_error',
    lane_id: laneId,
    error: String((payload && payload.error) || out.stderr || out.stdout || 'rust_anchor_failed').trim().slice(0, 260)
  };
}

function createLaneModule(laneId) {
  const normalized = String(laneId || '').trim().toUpperCase();
  function buildAnchor() {
    return runRustAnchor(normalized);
  }
  function verifyAnchor() {
    const row = buildAnchor();
    return row && row.ok === true && String(row.lane_id || '') === normalized;
  }
  return {
    LANE_ID: normalized,
    buildAnchor,
    verifyAnchor
  };
}

module.exports = {
  createLaneModule,
  runRustAnchor
};
