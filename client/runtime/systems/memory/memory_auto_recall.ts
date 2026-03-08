#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { runOpsDomainCommand } = require('../../lib/spine_conduit_bridge');
const { resolveClientState } = require('../../lib/runtime_path_registry');
const { buildTagMemoryMatrix, MATRIX_JSON_PATH } = require('./memory_matrix.js');

const CLIENT_ROOT = path.resolve(__dirname, '..', '..');
const WORKSPACE_ROOT = path.resolve(CLIENT_ROOT, '..');
const POLICY_PATH = process.env.MEMORY_AUTO_RECALL_POLICY_PATH
  ? path.resolve(String(process.env.MEMORY_AUTO_RECALL_POLICY_PATH))
  : path.join(CLIENT_ROOT, 'config', 'memory_auto_recall_policy.json');
const EVENTS_PATH = process.env.MEMORY_AUTO_RECALL_EVENTS_PATH
  ? path.resolve(String(process.env.MEMORY_AUTO_RECALL_EVENTS_PATH))
  : resolveClientState(WORKSPACE_ROOT, 'memory/auto_recall/events.jsonl');
const LATEST_PATH = process.env.MEMORY_AUTO_RECALL_LATEST_PATH
  ? path.resolve(String(process.env.MEMORY_AUTO_RECALL_LATEST_PATH))
  : resolveClientState(WORKSPACE_ROOT, 'memory/auto_recall/latest.json');

const DEFAULT_POLICY = {
  enabled: true,
  dry_run: false,
  min_shared_tags: 1,
  max_matches: 3,
  max_matrix_age_ms: 20 * 60 * 1000,
  enqueue_to_attention: true,
  summary_max_chars: 180,
  recall_window_days: 90,
  min_priority_score: 8
};

function cleanText(v: unknown, maxLen = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function normalizeTag(v: unknown) {
  return cleanText(v, 80)
    .toLowerCase()
    .replace(/^#/, '')
    .replace(/[^a-z0-9_-]/g, '');
}

function normalizeNodeId(v: unknown) {
  const s = cleanText(v, 160).replace(/`/g, '');
  return /^[A-Za-z0-9._-]+$/.test(s) ? s : '';
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

function parseNumber(v: unknown, fallback: number) {
  const n = Number(v);
  return Number.isFinite(n) ? n : fallback;
}

function clamp(v: number, min: number, max: number) {
  if (!Number.isFinite(v)) return min;
  return Math.max(min, Math.min(max, v));
}

function ensureDir(absDir: string) {
  fs.mkdirSync(absDir, { recursive: true });
}

function readJsonSafe(filePath: string, fallback: any = null) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function writeJson(filePath: string, payload: any) {
  ensureDir(path.dirname(filePath));
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function appendJsonl(filePath: string, payload: any) {
  ensureDir(path.dirname(filePath));
  fs.appendFileSync(filePath, `${JSON.stringify(payload)}\n`, 'utf8');
}

function relPath(absPath: string) {
  const rel = path.relative(WORKSPACE_ROOT, absPath).replace(/\\/g, '/');
  return rel.startsWith('..') ? absPath : rel;
}

function loadPolicy() {
  const raw = readJsonSafe(POLICY_PATH, {});
  return {
    enabled: parseBool(raw && raw.enabled, DEFAULT_POLICY.enabled),
    dry_run: parseBool(raw && raw.dry_run, DEFAULT_POLICY.dry_run),
    min_shared_tags: clamp(Math.round(parseNumber(raw && raw.min_shared_tags, DEFAULT_POLICY.min_shared_tags)), 1, 10),
    max_matches: clamp(Math.round(parseNumber(raw && raw.max_matches, DEFAULT_POLICY.max_matches)), 1, 20),
    max_matrix_age_ms: clamp(Math.round(parseNumber(raw && raw.max_matrix_age_ms, DEFAULT_POLICY.max_matrix_age_ms)), 60000, 24 * 60 * 60 * 1000),
    enqueue_to_attention: parseBool(raw && raw.enqueue_to_attention, DEFAULT_POLICY.enqueue_to_attention),
    summary_max_chars: clamp(Math.round(parseNumber(raw && raw.summary_max_chars, DEFAULT_POLICY.summary_max_chars)), 60, 400),
    recall_window_days: clamp(Math.round(parseNumber(raw && raw.recall_window_days, DEFAULT_POLICY.recall_window_days)), 1, 365),
    min_priority_score: clamp(parseNumber(raw && raw.min_priority_score, DEFAULT_POLICY.min_priority_score), 0, 100)
  };
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

function normalizeTags(tags: unknown) {
  if (!Array.isArray(tags)) return [];
  const out: string[] = [];
  for (const raw of tags) {
    const tag = normalizeTag(raw);
    if (!tag) continue;
    if (!out.includes(tag)) out.push(tag);
  }
  return out;
}

function loadMatrix(policy: any) {
  const payload = readJsonSafe(MATRIX_JSON_PATH, null);
  const generatedMs = Date.parse(String(payload && payload.generated_at || ''));
  const stale = !Number.isFinite(generatedMs) || (Date.now() - generatedMs > Number(policy.max_matrix_age_ms || 0));
  if (payload && payload.ok !== false && stale !== true) return payload;
  const rebuilt = buildTagMemoryMatrix({ apply: true, reason: 'memory_auto_recall_refresh' });
  if (rebuilt && rebuilt.ok === true) return rebuilt;
  return payload;
}

function tagsToMap(matrix: any) {
  const out = new Map<string, any>();
  const tags = Array.isArray(matrix && matrix.tags) ? matrix.tags : [];
  for (const row of tags) {
    const tag = normalizeTag(row && row.tag);
    if (!tag) continue;
    out.set(tag, row);
  }
  return out;
}

function overlapCount(a: string[], b: string[]) {
  const setB = new Set(b);
  let count = 0;
  for (const tag of a) {
    if (setB.has(tag)) count += 1;
  }
  return count;
}

function scoreCandidate(sharedCount: number, candidate: any) {
  const priority = Number(candidate && candidate.priority_score || 0);
  const dream = Number(candidate && candidate.dream_score || 0);
  const recency = Number(candidate && candidate.recency_score || 0);
  return (sharedCount * 50) + (priority * 0.85) + (dream * 12) + (recency * 8);
}

function summarizeMatches(nodeId: string, matches: any[], maxChars: number) {
  const heads = matches.slice(0, 3).map((m) => `${m.node_id}(${m.shared_tags.join(',')})`);
  const summary = `memory_auto_recall: ${nodeId} reminds me of ${heads.join(' | ')}`;
  return cleanText(summary, maxChars);
}

function buildAttentionEvent(sourceNode: any, matches: any[], policy: any) {
  const ts = nowIso();
  const attentionKeySeed = `${sourceNode.node_id}|${matches.map((m) => m.node_id).join('|')}|${ts.slice(0, 13)}`;
  const attentionKey = crypto.createHash('sha256').update(attentionKeySeed, 'utf8').digest('hex').slice(0, 20);
  return {
    ts,
    type: 'attention_event',
    source: 'memory_auto_recall',
    source_type: 'memory_auto_recall',
    severity: 'info',
    priority: 33,
    summary: summarizeMatches(sourceNode.node_id, matches, Number(policy.summary_max_chars || 180)),
    attention_key: `memory_auto_recall:${attentionKey}`,
    node_id: sourceNode.node_id,
    node_tags: sourceNode.tags,
    matches: matches.map((m) => ({
      node_id: m.node_id,
      score: m.score,
      priority_score: m.priority_score,
      shared_tags: m.shared_tags,
      date: m.date || null,
      level: m.level_token || null,
      summary: m.summary || null,
      file: m.file || null
    }))
  };
}

async function enqueueAttention(event: any) {
  const encoded = Buffer.from(JSON.stringify(event), 'utf8').toString('base64');
  const out = await runOpsDomainCommand(
    'attention-queue',
    ['enqueue', `--event-json-base64=${encoded}`, '--run-context=memory_auto_recall'],
    { skipRuntimeGate: true, runContext: 'memory_auto_recall' }
  );
  const payload = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  return {
    ok: out && out.ok === true,
    status: Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1,
    payload,
    decision: cleanText(payload && payload.decision, 40) || null,
    queued: payload && payload.queued === true,
    routed_via: cleanText(out && out.routed_via, 80) || 'conduit'
  };
}

function findMatches(sourceNode: any, matrix: any, policy: any) {
  const tagMap = tagsToMap(matrix);
  const sourceTags = normalizeTags(sourceNode && sourceNode.tags);
  const sourceNodeId = normalizeNodeId(sourceNode && sourceNode.node_id);
  const candidates = new Map<string, any>();

  for (const tag of sourceTags) {
    const row = tagMap.get(tag);
    const nodes = Array.isArray(row && row.nodes) ? row.nodes : [];
    for (const node of nodes) {
      const nodeId = normalizeNodeId(node && node.node_id);
      if (!nodeId || nodeId === sourceNodeId) continue;
      const nodeTags = normalizeTags(node && node.tags);
      const sharedCount = overlapCount(sourceTags, nodeTags);
      if (sharedCount < Number(policy.min_shared_tags || 1)) continue;
      const score = scoreCandidate(sharedCount, node);
      const existing = candidates.get(nodeId);
      const next = {
        node_id: nodeId,
        file: node.file || null,
        date: node.date || null,
        summary: node.summary || null,
        level_token: node.level_token || null,
        priority_score: Number(node.priority_score || 0),
        score,
        shared_tags: sourceTags.filter((tagValue) => nodeTags.includes(tagValue))
      };
      if (!existing || Number(existing.score || 0) < Number(next.score || 0)) {
        candidates.set(nodeId, next);
      }
    }
  }

  return Array.from(candidates.values())
    .filter((row) => Number(row.priority_score || 0) >= Number(policy.min_priority_score || 0))
    .sort((a, b) => Number(b.score || 0) - Number(a.score || 0) || String(a.node_id).localeCompare(String(b.node_id)))
    .slice(0, Number(policy.max_matches || 3));
}

async function processMemoryFiled(sourceNode: any, opts: any = {}) {
  const nodeId = normalizeNodeId(sourceNode && sourceNode.node_id);
  const tags = normalizeTags(sourceNode && sourceNode.tags);
  const policy = loadPolicy();
  const dryRun = opts && typeof opts.dryRun === 'boolean' ? opts.dryRun : parseBool(process.env.MEMORY_AUTO_RECALL_DRY_RUN, policy.dry_run);

  if (!nodeId || tags.length === 0) {
    const out = {
      ok: false,
      type: 'memory_auto_recall',
      reason: 'missing_node_or_tags',
      node_id: nodeId || null,
      tags,
      ts: nowIso()
    };
    appendJsonl(EVENTS_PATH, out);
    writeJson(LATEST_PATH, out);
    return out;
  }

  if (policy.enabled !== true) {
    const out = {
      ok: true,
      type: 'memory_auto_recall',
      skipped: true,
      reason: 'disabled',
      node_id: nodeId,
      tags,
      ts: nowIso()
    };
    appendJsonl(EVENTS_PATH, out);
    writeJson(LATEST_PATH, out);
    return out;
  }

  const matrix = loadMatrix(policy);
  if (!matrix || matrix.ok === false) {
    const out = {
      ok: false,
      type: 'memory_auto_recall',
      reason: 'matrix_unavailable',
      node_id: nodeId,
      tags,
      ts: nowIso()
    };
    appendJsonl(EVENTS_PATH, out);
    writeJson(LATEST_PATH, out);
    return out;
  }

  const matches = findMatches({ node_id: nodeId, tags }, matrix, policy);
  if (matches.length === 0) {
    const out = {
      ok: true,
      type: 'memory_auto_recall',
      skipped: true,
      reason: 'no_matches',
      node_id: nodeId,
      tags,
      ts: nowIso()
    };
    appendJsonl(EVENTS_PATH, out);
    writeJson(LATEST_PATH, out);
    return out;
  }

  let attention = {
    ok: true,
    skipped: true,
    reason: 'dry_run_or_queue_disabled',
    queued: false,
    routed_via: 'none'
  };

  if (policy.enqueue_to_attention === true && dryRun !== true) {
    const event = buildAttentionEvent({ node_id: nodeId, tags }, matches, policy);
    attention = await enqueueAttention(event);
  }

  const out = {
    ok: true,
    type: 'memory_auto_recall',
    ts: nowIso(),
    node_id: nodeId,
    tags,
    matches,
    match_count: matches.length,
    dry_run: dryRun,
    matrix_path: relPath(MATRIX_JSON_PATH),
    attention
  };
  appendJsonl(EVENTS_PATH, out);
  writeJson(LATEST_PATH, out);
  return out;
}

function status() {
  const latest = readJsonSafe(LATEST_PATH, null);
  return {
    ok: true,
    type: 'memory_auto_recall_status',
    policy: loadPolicy(),
    latest,
    paths: {
      events: relPath(EVENTS_PATH),
      latest: relPath(LATEST_PATH),
      matrix: relPath(MATRIX_JSON_PATH)
    }
  };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'status', 40).toLowerCase() || 'status';
  if (cmd === 'status') {
    process.stdout.write(`${JSON.stringify(status(), null, 2)}\n`);
    process.exit(0);
  }
  if (cmd === 'filed' || cmd === 'emit') {
    const nodeId = normalizeNodeId(args['node-id'] || args.node_id || args.node || '');
    const tags = cleanText(args.tags || '', 400)
      .split(',')
      .map((token) => normalizeTag(token))
      .filter(Boolean);
    const out = await processMemoryFiled({
      node_id: nodeId,
      tags,
      source: cleanText(args.source || 'manual', 80) || 'manual'
    }, {
      dryRun: parseBool(args['dry-run'], false)
    });
    process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
    process.exit(out && out.ok === true ? 0 : 1);
  }

  process.stdout.write(`${JSON.stringify({ ok: false, reason: `unknown_command:${cmd}` }, null, 2)}\n`);
  process.exit(1);
}

if (require.main === module) {
  main().catch((err) => {
    process.stdout.write(`${JSON.stringify({
      ok: false,
      type: 'memory_auto_recall',
      reason: cleanText(err && err.message ? err.message : err, 220),
      ts: nowIso()
    }, null, 2)}\n`);
    process.exit(1);
  });
}

module.exports = {
  processMemoryFiled,
  status
};
