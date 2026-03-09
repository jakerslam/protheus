#!/usr/bin/env node
// @ts-nocheck
'use strict';
export {};

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::memory-ambient (authoritative)
// TypeScript compatibility shim only.

const path = require('path');
const { runMemoryAmbientCommand } = require('../../lib/spine_conduit_bridge');
const legacy = require('./legacy/memory_matrix_legacy.js');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');

const MATRIX_JSON_PATH = process.env.MEMORY_MATRIX_JSON_PATH
  ? path.resolve(String(process.env.MEMORY_MATRIX_JSON_PATH))
  : path.join(ROOT, 'client', 'runtime', 'local', 'state', 'memory', 'matrix', 'tag_memory_matrix.json');
const MATRIX_MD_PATH = process.env.MEMORY_MATRIX_MD_PATH
  ? path.resolve(String(process.env.MEMORY_MATRIX_MD_PATH))
  : path.join(ROOT, 'client', 'memory', 'TAG_MEMORY_MATRIX.md');

function parseArgs(argv = []) {
  const out = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const token = String(argv[i] || '').trim();
    if (!token.startsWith('--')) {
      out._.push(token);
      continue;
    }
    const idx = token.indexOf('=');
    if (idx > 2) {
      out[token.slice(2, idx)] = token.slice(idx + 1);
      continue;
    }
    const key = token.slice(2);
    const next = String(argv[i + 1] || '');
    if (next && !next.startsWith('--')) {
      out[key] = next;
      i += 1;
      continue;
    }
    out[key] = '1';
  }
  return out;
}

function parseBool(v, fallback = false) {
  const raw = String(v == null ? '' : v).trim().toLowerCase();
  if (!raw) return fallback;
  if (['1', 'true', 'yes', 'on'].includes(raw)) return true;
  if (['0', 'false', 'no', 'off'].includes(raw)) return false;
  return fallback;
}

function toAmbientArgs(argv = []) {
  const parsed = parseArgs(argv);
  const cmd = String(parsed._[0] || 'run').trim().toLowerCase();
  const tail = (cmd === 'run' || cmd === 'build' || cmd === 'status') ? parsed._.slice(1) : parsed._;
  const action = cmd === 'status' ? 'status' : 'run';
  return ['run', 'memory-matrix', `--action=${action}`, ...tail, ...argv.filter((token) => String(token).startsWith('--'))];
}

function legacyFallback(argv = []) {
  const parsed = parseArgs(argv);
  const cmd = String(parsed._[0] || 'run').trim().toLowerCase();
  if (cmd === 'status') {
    const payload = legacy.status();
    return { ok: payload && payload.ok !== false, status: payload && payload.ok !== false ? 0 : 1, payload, stderr: '' };
  }
  if (cmd === 'run' || cmd === 'build') {
    const payload = legacy.buildTagMemoryMatrix({
      apply: parseBool(parsed.apply, true),
      reason: String(parsed.reason || 'manual').slice(0, 120)
    });
    return { ok: payload && payload.ok === true, status: payload && payload.ok === true ? 0 : 1, payload, stderr: '' };
  }
  const payload = { ok: false, reason: `unknown_command:${cmd}` };
  return { ok: false, status: 1, payload, stderr: '' };
}

function payloadLooksValid(payload) {
  if (!payload || typeof payload !== 'object') return false;
  const t = String(payload.type || '');
  return t === 'tag_memory_matrix' || t === 'tag_memory_matrix_status';
}

async function run(args = [], opts = {}) {
  const mapped = toAmbientArgs(args);
  try {
    const out = await runMemoryAmbientCommand(mapped, {
      runContext: 'memory_matrix_wrapper',
      skipRuntimeGate: true,
      timeoutMs: Number(process.env.PROTHEUS_MEMORY_MATRIX_TIMEOUT_MS || 60000),
      stdioTimeoutMs: Number(process.env.PROTHEUS_MEMORY_MATRIX_STDIO_TIMEOUT_MS || 15000),
      ...opts
    });
    if (out && out.ok === true && payloadLooksValid(out.payload) && out.payload.ok !== false) {
      return out;
    }
  } catch {
    // compatibility fallback below
  }
  return legacyFallback(args);
}

function buildTagMemoryMatrix(opts = {}) {
  return legacy.buildTagMemoryMatrix(opts);
}

function buildMatrixPayload(opts = {}) {
  return legacy.buildMatrixPayload(opts);
}

function status() {
  return legacy.status();
}

if (require.main === module) {
  process.env.PROTHEUS_CONDUIT_STARTUP_PROBE = '0';
  process.env.PROTHEUS_CONDUIT_COMPAT_FALLBACK = '0';
  run(process.argv.slice(2))
    .then((out) => {
      if (out && out.payload) {
        process.stdout.write(`${JSON.stringify(out.payload, null, 2)}\n`);
      }
      if (out && out.stderr) {
        process.stderr.write(String(out.stderr));
        if (!String(out.stderr).endsWith('\n')) process.stderr.write('\n');
      }
      process.exit(Number.isFinite(out && out.status) ? Number(out.status) : 1);
    })
    .catch((error) => {
      process.stdout.write(
        `${JSON.stringify({ ok: false, type: 'memory_matrix_wrapper_error', error: String(error && error.message ? error.message : error) })}\n`
      );
      process.exit(1);
    });
}

module.exports = {
  MATRIX_JSON_PATH,
  MATRIX_MD_PATH,
  run,
  status,
  buildTagMemoryMatrix,
  buildMatrixPayload
};
