#!/usr/bin/env node
'use strict';

const path = require('path');
const fs = require('fs');
const crypto = require('crypto');
const { runMemoryAmbientCommand } = require('../../lib/spine_conduit_bridge');

const ROOT = path.resolve(__dirname, '..', '..');

function readJson(filePath, fallback = null) {
  try {
    if (!filePath || !fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function writeJson(filePath, payload) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function parseArgs(argv) {
  const out = { _: [], memory_arg: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const token = String(argv[i] || '');
    if (!token.startsWith('--')) {
      out._.push(token);
      continue;
    }
    const idx = token.indexOf('=');
    if (idx >= 0) {
      const key = token.slice(2, idx);
      const value = token.slice(idx + 1);
      if (key === 'memory-arg') out.memory_arg.push(value);
      else out[key] = value;
      continue;
    }
    const key = token.slice(2);
    const next = argv[i + 1];
    if (next != null && !String(next).startsWith('--')) {
      if (key === 'memory-arg') out.memory_arg.push(String(next));
      else out[key] = String(next);
      i += 1;
      continue;
    }
    out[key] = true;
  }
  return out;
}

function toBool(v, fallback = false) {
  const raw = String(v == null ? '' : v).trim().toLowerCase();
  if (!raw) return fallback;
  if (['1', 'true', 'yes', 'on'].includes(raw)) return true;
  if (['0', 'false', 'no', 'off'].includes(raw)) return false;
  return fallback;
}

function parseMemoryArgs(memoryArgs = []) {
  const out = {};
  for (const row of memoryArgs) {
    const token = String(row || '').trim();
    if (!token.startsWith('--')) continue;
    const idx = token.indexOf('=');
    if (idx < 0) continue;
    out[token.slice(2, idx)] = token.slice(idx + 1);
  }
  return out;
}

function memoryStorePath() {
  const raw = String(process.env.PROTHEUS_MEMORY_DB_PATH || '').trim();
  if (raw) return path.isAbsolute(raw) ? raw : path.join(ROOT, raw);
  return path.join(ROOT, 'local', 'state', 'memory', 'ambient', 'runtime_memory.json');
}

function hash(payload) {
  return crypto.createHash('sha256').update(JSON.stringify(payload)).digest('hex');
}

function localCompatRun(argv = []) {
  const args = parseArgs(argv);
  const command = String(args._[0] || 'status').trim().toLowerCase();
  const dbPath = memoryStorePath();
  const state = readJson(dbPath, { rows: [] }) || { rows: [] };

  if (command === 'run') {
    const memoryCommand = String(args['memory-command'] || '').trim().toLowerCase();
    const memArgs = parseMemoryArgs(args.memory_arg);
    if (memoryCommand === 'ingest') {
      const row = {
        id: String(memArgs.id || `memory://${Date.now()}`),
        content: String(memArgs.content || ''),
        tags: String(memArgs.tags || '').split(',').map((v) => v.trim()).filter(Boolean),
        ts: new Date().toISOString()
      };
      state.rows.push(row);
      writeJson(dbPath, state);
      const payload = {
        ok: true,
        type: 'memory_ambient_run',
        memory_command: 'ingest',
        stored: 1,
        rust_authoritative: true,
        receipt_hash: null
      };
      payload.receipt_hash = hash(payload);
      return { ok: true, status: 0, payload, stderr: '' };
    }
    if (memoryCommand === 'recall') {
      const query = String(memArgs.query || '').toLowerCase();
      const limit = Math.max(1, Number(memArgs.limit || 5) || 5);
      const hits = state.rows
        .filter((row) => String(row.content || '').toLowerCase().includes(query))
        .slice(0, limit);
      const payload = {
        ok: true,
        type: 'memory_ambient_run',
        memory_command: 'recall',
        hits,
        rust_authoritative: true,
        receipt_hash: null
      };
      payload.receipt_hash = hash(payload);
      return { ok: true, status: 0, payload, stderr: '' };
    }
  }

  const payload = {
    ok: true,
    type: 'memory_ambient_status',
    ts: new Date().toISOString(),
    rows: Array.isArray(state.rows) ? state.rows.length : 0,
    rust_authoritative: true,
    ambient_mode_active: toBool(process.env.MECH_SUIT_MODE_FORCE, false)
  };
  payload.receipt_hash = hash(payload);
  return { ok: true, status: 0, payload, stderr: '' };
}

function normalizeGateDegraded(out) {
  if (!out || !out.payload || out.payload.gate_active !== true) return out;
  return {
    ...out,
    ok: true,
    status: 0,
    payload: {
      ok: true,
      blocked: true,
      type: 'memory_ambient_status',
      degraded: true,
      degraded_reason: 'conduit_runtime_gate_active',
      gate_active: true,
      gate_reason: String(out.payload.reason || '').slice(0, 240) || 'conduit_runtime_gate_active',
      routed_via: 'conduit'
    },
    stderr: ''
  };
}

async function run(args = [], opts = {}) {
  if (toBool(process.env.PROTHEUS_MEMORY_AMBIENT_LOCAL_COMPAT, false)) {
    return localCompatRun(args);
  }
  const routed = Array.isArray(args) && args.length > 0 ? args : ['status'];
  const out = await runMemoryAmbientCommand(routed, {
    cwdHint: opts.cwdHint || ROOT
  });
  return normalizeGateDegraded(out);
}

async function main() {
  const out = await run(process.argv.slice(2));
  if (out.payload) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  } else if (out.stdout) {
    process.stdout.write(String(out.stdout));
  }
  if (out.stderr) {
    process.stderr.write(String(out.stderr));
    if (!String(out.stderr).endsWith('\n')) process.stderr.write('\n');
  }
  process.exit(Number.isFinite(out.status) ? Number(out.status) : 1);
}

if (require.main === module) {
  main().catch((err) => {
    process.stderr.write(`${String(err && err.message ? err.message : err)}\n`);
    process.exit(1);
  });
}

module.exports = {
  run
};
