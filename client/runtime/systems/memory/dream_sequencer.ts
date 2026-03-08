#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');
const { resolveClientState } = require('../../../lib/runtime_path_registry');
const { buildTagMemoryMatrix, status: matrixStatus, MATRIX_JSON_PATH } = require('./memory_matrix.js');

const CLIENT_ROOT = path.resolve(__dirname, '..', '..');
const WORKSPACE_ROOT = path.resolve(CLIENT_ROOT, '..');
const SEQUENCER_STATE_PATH = process.env.DREAM_SEQUENCER_STATE_PATH
  ? path.resolve(String(process.env.DREAM_SEQUENCER_STATE_PATH))
  : resolveClientState(WORKSPACE_ROOT, 'memory/dream_sequencer/latest.json');
const SEQUENCER_LEDGER_PATH = process.env.DREAM_SEQUENCER_LEDGER_PATH
  ? path.resolve(String(process.env.DREAM_SEQUENCER_LEDGER_PATH))
  : resolveClientState(WORKSPACE_ROOT, 'memory/dream_sequencer/runs.jsonl');

function cleanText(v: unknown, maxLen = 220) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function nowIso() {
  return new Date().toISOString();
}

function parseBool(v: unknown, fallback = false) {
  const s = cleanText(v, 20).toLowerCase();
  if (!s) return fallback;
  if (['1', 'true', 'yes', 'on'].includes(s)) return true;
  if (['0', 'false', 'no', 'off'].includes(s)) return false;
  return fallback;
}

function ensureDir(absDir: string) {
  fs.mkdirSync(absDir, { recursive: true });
}

function writeJson(filePath: string, payload: any) {
  ensureDir(path.dirname(filePath));
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function appendJsonl(filePath: string, payload: any) {
  ensureDir(path.dirname(filePath));
  fs.appendFileSync(filePath, `${JSON.stringify(payload)}\n`, 'utf8');
}

function readJsonSafe(filePath: string, fallback = null) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function relPath(absPath: string) {
  const rel = path.relative(WORKSPACE_ROOT, absPath).replace(/\\/g, '/');
  return rel.startsWith('..') ? absPath : rel;
}

function parseArgs(argv: string[]) {
  const out: Record<string, any> = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const token = String(argv[i] || '').trim();
    if (!token) continue;
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
    } else {
      out[key] = '1';
    }
  }
  return out;
}

function summarizeTopTags(matrixPayload: any, maxTags = 8) {
  const tags = Array.isArray(matrixPayload && matrixPayload.tags) ? matrixPayload.tags : [];
  return tags.slice(0, Math.max(1, maxTags)).map((row: any) => ({
    tag: cleanText(row && row.tag, 80),
    tag_priority: Number(row && row.tag_priority || 0),
    node_count: Number(row && row.node_count || 0),
    top_nodes: Array.isArray(row && row.node_ids) ? row.node_ids.slice(0, 5) : []
  }));
}

function runDreamSequencer(opts: any = {}) {
  const reason = cleanText(opts.reason || 'dream_cycle', 120) || 'dream_cycle';
  const apply = opts && typeof opts.apply === 'boolean' ? opts.apply : true;
  const topTagCount = Math.max(1, Math.min(50, Number(opts.topTagCount || 12) || 12));

  const matrixRun = buildTagMemoryMatrix({
    apply,
    reason: `dream_sequencer:${reason}`
  });

  if (!matrixRun || matrixRun.ok !== true) {
    const fail = {
      ok: false,
      type: 'dream_sequencer',
      reason: 'matrix_build_failed',
      matrix: matrixRun || null,
      ts: nowIso()
    };
    appendJsonl(SEQUENCER_LEDGER_PATH, fail);
    return fail;
  }

  const topTags = summarizeTopTags(matrixRun, topTagCount);
  const payload = {
    ok: true,
    type: 'dream_sequencer',
    ts: nowIso(),
    reason,
    applied: apply,
    matrix_path: relPath(MATRIX_JSON_PATH),
    stats: matrixRun.stats || {},
    top_tags: topTags
  };

  if (apply) {
    writeJson(SEQUENCER_STATE_PATH, payload);
  }
  appendJsonl(SEQUENCER_LEDGER_PATH, payload);
  return payload;
}

function statusDreamSequencer() {
  const latest = readJsonSafe(SEQUENCER_STATE_PATH, null);
  const matrix = matrixStatus();
  return {
    ok: true,
    type: 'dream_sequencer_status',
    latest,
    matrix,
    sequencer_state_path: relPath(SEQUENCER_STATE_PATH),
    sequencer_ledger_path: relPath(SEQUENCER_LEDGER_PATH)
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'run', 32).toLowerCase() || 'run';
  if (cmd === 'status') {
    process.stdout.write(`${JSON.stringify(statusDreamSequencer(), null, 2)}\n`);
    process.exit(0);
  }
  if (cmd === 'run') {
    const out = runDreamSequencer({
      apply: parseBool(args.apply, true),
      reason: cleanText(args.reason || 'manual', 120) || 'manual',
      topTagCount: Number(args['top-tags'] || args.top_tags || 12)
    });
    process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
    process.exit(out && out.ok === true ? 0 : 1);
  }
  process.stdout.write(`${JSON.stringify({ ok: false, reason: `unknown_command:${cmd}` }, null, 2)}\n`);
  process.exit(1);
}

if (require.main === module) {
  main();
}

module.exports = {
  runDreamSequencer,
  statusDreamSequencer
};
