#!/usr/bin/env node
// @ts-nocheck
'use strict';
export {};

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::memory-ambient (authoritative)
// TypeScript compatibility shim only.

const { runMemoryAmbientCommand } = require('../../lib/spine_conduit_bridge');
const legacy = require('./legacy/memory_auto_recall_legacy.js');

function cleanText(v, maxLen = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

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

function normalizeTag(v) {
  return cleanText(v, 80)
    .toLowerCase()
    .replace(/^#/, '')
    .replace(/[^a-z0-9_-]/g, '');
}

function normalizeNodeId(v) {
  const s = cleanText(v, 160).replace(/`/g, '');
  return /^[A-Za-z0-9._-]+$/.test(s) ? s : '';
}

function toAmbientArgs(argv = []) {
  const parsed = parseArgs(argv);
  const cmd = String(parsed._[0] || 'status').trim().toLowerCase();
  const tail = (cmd === 'status' || cmd === 'filed' || cmd === 'emit') ? parsed._.slice(1) : parsed._;
  const action = (cmd === 'filed' || cmd === 'emit') ? 'filed' : 'status';
  return ['run', 'memory-auto-recall', `--action=${action}`, ...tail, ...argv.filter((token) => String(token).startsWith('--'))];
}

async function legacyFallback(argv = []) {
  const parsed = parseArgs(argv);
  const cmd = String(parsed._[0] || 'status').trim().toLowerCase();
  if (cmd === 'status') {
    const payload = legacy.status();
    return { ok: payload && payload.ok !== false, status: payload && payload.ok !== false ? 0 : 1, payload, stderr: '' };
  }

  if (cmd === 'filed' || cmd === 'emit') {
    const payload = await legacy.processMemoryFiled(
      {
        node_id: normalizeNodeId(parsed['node-id'] || parsed.node_id || parsed.node || ''),
        tags: cleanText(parsed.tags || '', 400)
          .split(',')
          .map((token) => normalizeTag(token))
          .filter(Boolean),
        source: cleanText(parsed.source || 'manual', 80) || 'manual'
      },
      { dryRun: parseBool(parsed['dry-run'], false) }
    );
    return { ok: payload && payload.ok === true, status: payload && payload.ok === true ? 0 : 1, payload, stderr: '' };
  }

  const payload = { ok: false, reason: `unknown_command:${cmd}` };
  return { ok: false, status: 1, payload, stderr: '' };
}

function payloadLooksValid(payload) {
  if (!payload || typeof payload !== 'object') return false;
  const t = String(payload.type || '');
  return t === 'memory_auto_recall' || t === 'memory_auto_recall_status';
}

async function run(args = [], opts = {}) {
  const mapped = toAmbientArgs(args);
  try {
    const out = await runMemoryAmbientCommand(mapped, {
      runContext: 'memory_auto_recall_wrapper',
      skipRuntimeGate: true,
      timeoutMs: Number(process.env.PROTHEUS_MEMORY_AUTO_RECALL_TIMEOUT_MS || 60000),
      stdioTimeoutMs: Number(process.env.PROTHEUS_MEMORY_AUTO_RECALL_STDIO_TIMEOUT_MS || 15000),
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

async function processMemoryFiled(sourceNode, opts = {}) {
  return legacy.processMemoryFiled(sourceNode, opts);
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
      process.stdout.write(`${JSON.stringify({ ok: false, type: 'memory_auto_recall_wrapper_error', reason: cleanText(error && error.message ? error.message : error, 220) }, null, 2)}\n`);
      process.exit(1);
    });
}

module.exports = {
  run,
  processMemoryFiled,
  status
};
