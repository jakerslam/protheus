#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { resolveClientState } = require('../../lib/runtime_path_registry');

const CLIENT_ROOT = path.resolve(__dirname, '..', '..');
const WORKSPACE_ROOT = path.resolve(CLIENT_ROOT, '..');
const MEMORY_DIR = process.env.MEMORY_MATRIX_MEMORY_DIR
  ? path.resolve(String(process.env.MEMORY_MATRIX_MEMORY_DIR))
  : path.join(CLIENT_ROOT, 'memory');
const MEMORY_INDEX_PATH = process.env.MEMORY_MATRIX_INDEX_PATH
  ? path.resolve(String(process.env.MEMORY_MATRIX_INDEX_PATH))
  : path.join(MEMORY_DIR, 'MEMORY_INDEX.md');
const TAGS_INDEX_PATH = process.env.MEMORY_MATRIX_TAGS_PATH
  ? path.resolve(String(process.env.MEMORY_MATRIX_TAGS_PATH))
  : path.join(MEMORY_DIR, 'TAGS_INDEX.md');
const MATRIX_JSON_PATH = process.env.MEMORY_MATRIX_JSON_PATH
  ? path.resolve(String(process.env.MEMORY_MATRIX_JSON_PATH))
  : resolveClientState(WORKSPACE_ROOT, 'memory/matrix/tag_memory_matrix.json');
const MATRIX_MD_PATH = process.env.MEMORY_MATRIX_MD_PATH
  ? path.resolve(String(process.env.MEMORY_MATRIX_MD_PATH))
  : path.join(MEMORY_DIR, 'TAG_MEMORY_MATRIX.md');
const DREAM_IDLE_DIR = process.env.MEMORY_MATRIX_IDLE_DIR
  ? path.resolve(String(process.env.MEMORY_MATRIX_IDLE_DIR))
  : resolveClientState(WORKSPACE_ROOT, 'memory/dreams/idle');
const DREAM_REM_DIR = process.env.MEMORY_MATRIX_REM_DIR
  ? path.resolve(String(process.env.MEMORY_MATRIX_REM_DIR))
  : resolveClientState(WORKSPACE_ROOT, 'memory/dreams/rem');
const CONVERSATION_NODES_PATH = process.env.MEMORY_MATRIX_CONVERSATION_PATH
  ? path.resolve(String(process.env.MEMORY_MATRIX_CONVERSATION_PATH))
  : resolveClientState(WORKSPACE_ROOT, 'memory/conversation_eye/nodes.jsonl');
const POLICY_PATH = process.env.MEMORY_MATRIX_POLICY_PATH
  ? path.resolve(String(process.env.MEMORY_MATRIX_POLICY_PATH))
  : path.join(CLIENT_ROOT, 'config', 'memory_matrix_policy.json');

const DEFAULT_POLICY = {
  enabled: true,
  max_nodes_per_tag: 1000,
  top_nodes_per_tag: 64,
  dream_window_days: 21,
  recency_half_life_days: 21,
  score_precision: 4,
  weights: {
    level: 0.56,
    recency: 0.29,
    dream: 0.15
  },
  level_weights: {
    node: 1.0,
    tag: 0.67,
    jot: 0.34
  }
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
  const s = cleanText(v, 128).replace(/`/g, '');
  return /^[A-Za-z0-9._-]+$/.test(s) ? s : '';
}

function nowIso() {
  return new Date().toISOString();
}

function readJsonSafe(filePath: string, fallback: any = null) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function readTextSafe(filePath: string) {
  try {
    if (!fs.existsSync(filePath)) return '';
    return fs.readFileSync(filePath, 'utf8');
  } catch {
    return '';
  }
}

function readJsonlSafe(filePath: string, maxRows = 10000) {
  if (!fs.existsSync(filePath)) return [];
  const raw = fs.readFileSync(filePath, 'utf8');
  const lines = raw.split(/\r?\n/).filter(Boolean);
  const rows = lines.slice(Math.max(0, lines.length - maxRows));
  const out: any[] = [];
  for (const line of rows) {
    try {
      out.push(JSON.parse(line));
    } catch {
      // ignore malformed row
    }
  }
  return out;
}

function ensureDir(absDir: string) {
  fs.mkdirSync(absDir, { recursive: true });
}

function relPath(absPath: string) {
  const rel = path.relative(WORKSPACE_ROOT, absPath).replace(/\\/g, '/');
  return rel.startsWith('..') ? absPath : rel;
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

function round(v: number, digits = 4) {
  const n = Number(v);
  if (!Number.isFinite(n)) return 0;
  const f = Math.pow(10, Math.max(0, digits));
  return Math.round(n * f) / f;
}

function loadPolicy() {
  const raw = readJsonSafe(POLICY_PATH, {});
  const weights = raw && raw.weights && typeof raw.weights === 'object' ? raw.weights : {};
  const levelWeights = raw && raw.level_weights && typeof raw.level_weights === 'object'
    ? raw.level_weights
    : {};
  return {
    enabled: parseBool(raw && raw.enabled, DEFAULT_POLICY.enabled),
    max_nodes_per_tag: clamp(Math.round(parseNumber(raw && raw.max_nodes_per_tag, DEFAULT_POLICY.max_nodes_per_tag)), 8, 5000),
    top_nodes_per_tag: clamp(Math.round(parseNumber(raw && raw.top_nodes_per_tag, DEFAULT_POLICY.top_nodes_per_tag)), 8, 500),
    dream_window_days: clamp(Math.round(parseNumber(raw && raw.dream_window_days, DEFAULT_POLICY.dream_window_days)), 1, 365),
    recency_half_life_days: clamp(Math.round(parseNumber(raw && raw.recency_half_life_days, DEFAULT_POLICY.recency_half_life_days)), 1, 365),
    score_precision: clamp(Math.round(parseNumber(raw && raw.score_precision, DEFAULT_POLICY.score_precision)), 2, 6),
    weights: {
      level: clamp(parseNumber(weights.level, DEFAULT_POLICY.weights.level), 0, 1),
      recency: clamp(parseNumber(weights.recency, DEFAULT_POLICY.weights.recency), 0, 1),
      dream: clamp(parseNumber(weights.dream, DEFAULT_POLICY.weights.dream), 0, 1)
    },
    level_weights: {
      node: clamp(parseNumber(levelWeights.node, DEFAULT_POLICY.level_weights.node), 0.1, 2),
      tag: clamp(parseNumber(levelWeights.tag, DEFAULT_POLICY.level_weights.tag), 0.1, 2),
      jot: clamp(parseNumber(levelWeights.jot, DEFAULT_POLICY.level_weights.jot), 0.1, 2)
    }
  };
}

function normalizeHeaderCell(v: string) {
  const s = cleanText(v, 80).toLowerCase().replace(/[^a-z0-9_]+/g, '_');
  if (s.includes('node_id')) return 'node_id';
  if (s.startsWith('uid')) return 'uid';
  if (s.startsWith('tags')) return 'tags';
  if (s.startsWith('file')) return 'file';
  if (s.startsWith('summary') || s.startsWith('title')) return 'summary';
  return s;
}

function parseTagsCell(v: string) {
  const out: string[] = [];
  for (const token of String(v || '').split(/[\s,]+/)) {
    const tag = normalizeTag(token);
    if (!tag) continue;
    if (!out.includes(tag)) out.push(tag);
  }
  return out;
}

function parseDateFromFileRef(v: string) {
  const m = String(v || '').match(/(\d{4}-\d{2}-\d{2})\.md$/);
  return m ? m[1] : null;
}

function parseMemoryIndex() {
  const text = readTextSafe(MEMORY_INDEX_PATH);
  const lines = text.split(/\r?\n/);
  const entries: any[] = [];
  let section = 'unknown';
  let headers: string[] | null = null;

  for (const rawLine of lines) {
    const line = String(rawLine || '');
    const trimmed = line.trim();
    if (!trimmed) continue;
    if (trimmed.startsWith('## ')) {
      section = cleanText(trimmed.slice(3), 80).toLowerCase();
      headers = null;
      continue;
    }
    if (!trimmed.startsWith('|')) continue;
    const cells = trimmed.split('|').slice(1, -1).map((cell) => cleanText(cell, 400));
    if (!cells.length) continue;
    if (cells.every((cell) => /^[-: ]+$/.test(cell))) continue;

    const normalized = cells.map(normalizeHeaderCell);
    if (normalized.includes('node_id') && normalized.includes('file')) {
      headers = normalized;
      continue;
    }
    if (!headers) continue;

    const row: Record<string, string> = {};
    for (let i = 0; i < headers.length; i += 1) row[headers[i]] = cells[i] || '';
    const nodeId = normalizeNodeId(row.node_id || '');
    if (!nodeId) continue;
    const file = cleanText(row.file || '', 260);
    const tags = parseTagsCell(row.tags || '');
    const date = parseDateFromFileRef(file);
    entries.push({
      node_id: nodeId,
      uid: cleanText(row.uid || '', 64) || null,
      tags,
      file,
      summary: cleanText(row.summary || '', 280) || null,
      date,
      section
    });
  }

  return entries;
}

function parseTagsIndexFallback() {
  const text = readTextSafe(TAGS_INDEX_PATH);
  const lines = text.split(/\r?\n/);
  const out = new Map<string, Set<string>>();
  for (const rawLine of lines) {
    const line = cleanText(rawLine, 10000);
    if (!line.startsWith('#') || !line.includes('→')) continue;
    const idx = line.indexOf('→');
    const tag = normalizeTag(line.slice(0, idx));
    if (!tag) continue;
    const rhs = line.slice(idx + 1);
    const nodeIds = rhs.split(',').map((part) => normalizeNodeId(part)).filter(Boolean);
    if (!out.has(tag)) out.set(tag, new Set());
    const set = out.get(tag);
    for (const nodeId of nodeIds) set.add(nodeId);
  }
  return out;
}

function levelFromToken(token: string, fallbackLevel: number) {
  const t = cleanText(token, 20).toLowerCase();
  if (t === 'node' || t === 'node1') return { level: 1, label: 'node' };
  if (t === 'tag' || t === 'tag2') return { level: 2, label: 'tag' };
  if (t === 'jot' || t === 'jot3') return { level: 3, label: 'jot' };
  return fallbackLevel <= 1 ? { level: 1, label: 'node' } : fallbackLevel === 2 ? { level: 2, label: 'tag' } : { level: 3, label: 'jot' };
}

function inferLevel(entry: any) {
  const explicitLevel = Number(entry && entry.level);
  if (Number.isFinite(explicitLevel) && explicitLevel >= 1 && explicitLevel <= 3) {
    return levelFromToken(String(entry.level_token || ''), Math.round(explicitLevel));
  }
  const nodeId = cleanText(entry && entry.node_id, 120).toLowerCase();
  const tags = Array.isArray(entry && entry.tags) ? entry.tags.map((tag: unknown) => normalizeTag(tag)) : [];
  if (nodeId.startsWith('jot-') || tags.includes('jot')) return { level: 3, label: 'jot' };
  if (nodeId.startsWith('tag-') || tags.includes('topic') || tags.includes('tag')) return { level: 2, label: 'tag' };
  return { level: 1, label: 'node' };
}

function loadConversationNodeSupplements() {
  const rows = readJsonlSafe(CONVERSATION_NODES_PATH, 12000);
  const out: any[] = [];
  for (const row of rows) {
    const nodeId = normalizeNodeId(row && row.node_id);
    if (!nodeId) continue;
    const tags = Array.isArray(row && row.tags) ? row.tags.map((tag: unknown) => normalizeTag(tag)).filter(Boolean) : [];
    const date = String(row && row.ts || '').slice(0, 10);
    out.push({
      node_id: nodeId,
      tags,
      file: 'local/state/memory/conversation_eye/nodes.jsonl',
      summary: cleanText(row && row.preview, 280) || cleanText(row && row.title, 280) || null,
      date: /^\d{4}-\d{2}-\d{2}$/.test(date) ? date : null,
      section: 'conversation_eye',
      level: Number(row && row.level),
      level_token: cleanText(row && row.level_token, 20) || null,
      dream_seed: false
    });
  }
  return out;
}

function parseNodeRef(value: unknown) {
  const s = cleanText(value, 512);
  if (!s) return '';
  const idx = s.lastIndexOf('#');
  if (idx >= 0) {
    return normalizeNodeId(s.slice(idx + 1));
  }
  return normalizeNodeId(s);
}

function parseDateFromDreamFile(name: string) {
  const m = String(name || '').match(/(\d{4}-\d{2}-\d{2})/);
  return m ? m[1] : null;
}

function loadDreamSignals(policy: any) {
  const nodeHits = new Map<string, number>();
  const tagHits = new Map<string, number>();
  const cutoffMs = Date.now() - (Number(policy.dream_window_days || 21) * 24 * 60 * 60 * 1000);

  function ingestToken(token: unknown) {
    const tag = normalizeTag(token);
    if (!tag) return;
    tagHits.set(tag, Number(tagHits.get(tag) || 0) + 1);
  }

  function ingestRef(ref: unknown) {
    const nodeId = parseNodeRef(ref);
    if (!nodeId) return;
    nodeHits.set(nodeId, Number(nodeHits.get(nodeId) || 0) + 1);
  }

  function shouldIngestByDate(fileName: string) {
    const d = parseDateFromDreamFile(fileName);
    if (!d) return true;
    const ms = Date.parse(`${d}T00:00:00.000Z`);
    return !Number.isFinite(ms) || ms >= cutoffMs;
  }

  for (const dirPath of [DREAM_IDLE_DIR, DREAM_REM_DIR]) {
    if (!fs.existsSync(dirPath)) continue;
    const files = fs.readdirSync(dirPath).filter((name: string) => name.endsWith('.json') || name.endsWith('.jsonl'));
    for (const file of files) {
      if (!shouldIngestByDate(file)) continue;
      const abs = path.join(dirPath, file);
      if (file.endsWith('.jsonl')) {
        const rows = readJsonlSafe(abs, 4000);
        for (const row of rows) {
          const seeds = Array.isArray(row && row.seeds) ? row.seeds : [];
          for (const seed of seeds) {
            ingestToken(seed && seed.token);
            const refs = Array.isArray(seed && seed.refs) ? seed.refs : [];
            for (const ref of refs) ingestRef(ref);
          }
          const quantized = Array.isArray(row && row.quantized) ? row.quantized : [];
          for (const q of quantized) ingestToken(q && q.token);
        }
        continue;
      }
      const payload = readJsonSafe(abs, null);
      if (!payload || typeof payload !== 'object') continue;
      const seeds = Array.isArray(payload.seeds) ? payload.seeds : [];
      for (const seed of seeds) {
        ingestToken(seed && seed.token);
        const refs = Array.isArray(seed && seed.refs) ? seed.refs : [];
        for (const ref of refs) ingestRef(ref);
      }
      const quantized = Array.isArray(payload.quantized) ? payload.quantized : [];
      for (const q of quantized) ingestToken(q && q.token);
    }
  }

  return { nodeHits, tagHits };
}

function recencyScore(dateStr: string, halfLifeDays: number) {
  const ms = Date.parse(`${String(dateStr || '')}T00:00:00.000Z`);
  if (!Number.isFinite(ms)) return 0.22;
  const ageDays = Math.max(0, (Date.now() - ms) / (24 * 60 * 60 * 1000));
  const lambda = Math.log(2) / Math.max(1, halfLifeDays);
  return Math.exp(-lambda * ageDays);
}

function scoreNode(entry: any, policy: any, dreamSignals: any) {
  const inferred = inferLevel(entry);
  const levelWeight = Number(policy.level_weights[inferred.label] || 0.2);
  const recency = recencyScore(entry && entry.date, Number(policy.recency_half_life_days || 21));
  const dreamNodeHits = Number(dreamSignals.nodeHits.get(entry.node_id) || 0);
  let dreamTagHits = 0;
  const tags = Array.isArray(entry && entry.tags) ? entry.tags : [];
  for (const tag of tags) dreamTagHits += Number(dreamSignals.tagHits.get(tag) || 0);
  const dream = clamp((dreamNodeHits * 0.45) + (Math.min(20, dreamTagHits) * 0.03), 0, 1.5);

  const base = (
    Number(policy.weights.level || 0) * levelWeight
    + Number(policy.weights.recency || 0) * recency
    + Number(policy.weights.dream || 0) * dream
  );

  const precision = Number(policy.score_precision || 4);
  return {
    level: inferred.level,
    level_token: `${inferred.label}${inferred.level}`,
    level_score: round(levelWeight, precision),
    recency_score: round(recency, precision),
    dream_score: round(dream, precision),
    dream_node_hits: dreamNodeHits,
    dream_tag_hits: dreamTagHits,
    priority_score: round(base * 100, precision)
  };
}

function hexIdForNode(nodeId: string) {
  const digest = crypto.createHash('sha256').update(String(nodeId || ''), 'utf8').digest('hex');
  return `0x${digest.slice(0, 12)}`;
}

function mergeEntries(baseEntries: any[], supplements: any[]) {
  const map = new Map<string, any>();
  for (const row of baseEntries || []) {
    const nodeId = normalizeNodeId(row && row.node_id);
    if (!nodeId) continue;
    map.set(nodeId, {
      ...row,
      node_id: nodeId,
      tags: Array.isArray(row && row.tags) ? row.tags.map((tag: unknown) => normalizeTag(tag)).filter(Boolean) : []
    });
  }
  for (const row of supplements || []) {
    const nodeId = normalizeNodeId(row && row.node_id);
    if (!nodeId) continue;
    if (!map.has(nodeId)) {
      map.set(nodeId, {
        ...row,
        node_id: nodeId,
        tags: Array.isArray(row && row.tags) ? row.tags.map((tag: unknown) => normalizeTag(tag)).filter(Boolean) : []
      });
      continue;
    }
    const cur = map.get(nodeId);
    const tags = new Set<string>([...(Array.isArray(cur.tags) ? cur.tags : []), ...(Array.isArray(row.tags) ? row.tags : [])].map(normalizeTag).filter(Boolean));
    map.set(nodeId, {
      ...cur,
      level: Number.isFinite(Number(row.level)) ? Number(row.level) : cur.level,
      level_token: row.level_token || cur.level_token || null,
      tags: Array.from(tags)
    });
  }
  return Array.from(map.values());
}

function attachTagsFallback(entries: any[]) {
  const map = new Map(entries.map((row) => [row.node_id, row]));
  const fallback = parseTagsIndexFallback();
  for (const [tag, nodeSet] of fallback.entries()) {
    for (const nodeId of nodeSet.values()) {
      if (!map.has(nodeId)) continue;
      const row = map.get(nodeId);
      const tags = Array.isArray(row.tags) ? row.tags.slice() : [];
      if (!tags.includes(tag)) tags.push(tag);
      row.tags = tags;
      map.set(nodeId, row);
    }
  }
  return Array.from(map.values());
}

function buildMatrixPayload(opts: any = {}) {
  const policy = loadPolicy();
  const reason = cleanText(opts.reason || 'manual', 120) || 'manual';
  if (policy.enabled !== true) {
    return {
      ok: true,
      skipped: true,
      reason: 'memory_matrix_disabled',
      policy,
      generated_at: nowIso()
    };
  }

  const indexEntries = parseMemoryIndex();
  const conversationSupplements = loadConversationNodeSupplements();
  const merged = attachTagsFallback(mergeEntries(indexEntries, conversationSupplements));
  const dreamSignals = loadDreamSignals(policy);

  const scored: any[] = [];
  for (const entry of merged) {
    const tags = Array.isArray(entry.tags) ? entry.tags.filter(Boolean) : [];
    if (tags.length === 0) continue;
    const score = scoreNode(entry, policy, dreamSignals);
    scored.push({
      node_id: entry.node_id,
      uid: entry.uid || null,
      file: entry.file || null,
      date: entry.date || null,
      summary: entry.summary || null,
      section: entry.section || null,
      tags,
      hex_id: hexIdForNode(entry.node_id),
      ...score
    });
  }

  const byTag = new Map<string, any[]>();
  for (const row of scored) {
    for (const tag of row.tags) {
      if (!byTag.has(tag)) byTag.set(tag, []);
      byTag.get(tag).push(row);
    }
  }

  const tags: any[] = [];
  for (const [tag, rows] of byTag.entries()) {
    const sorted = rows.slice().sort((a, b) => {
      const p = Number(b.priority_score || 0) - Number(a.priority_score || 0);
      if (Math.abs(p) > 1e-9) return p;
      const da = Date.parse(`${String(a.date || '')}T00:00:00.000Z`);
      const db = Date.parse(`${String(b.date || '')}T00:00:00.000Z`);
      const dr = (Number.isFinite(db) ? db : 0) - (Number.isFinite(da) ? da : 0);
      if (dr !== 0) return dr;
      return String(a.node_id).localeCompare(String(b.node_id));
    }).slice(0, Number(policy.max_nodes_per_tag || 1000));

    const top = sorted.slice(0, 3);
    const avgTop = top.length
      ? top.reduce((sum, row) => sum + Number(row.priority_score || 0), 0) / top.length
      : 0;
    const dreamTagHits = Number(dreamSignals.tagHits.get(tag) || 0);
    const tagPriority = round((Number(top[0] && top[0].priority_score || 0) * 0.7)
      + (avgTop * 0.2)
      + (Math.log(1 + dreamTagHits) * 4)
      + (Math.log(1 + sorted.length) * 2), 4);

    tags.push({
      tag,
      tag_priority: tagPriority,
      dream_tag_hits: dreamTagHits,
      node_count: sorted.length,
      node_ids: sorted.map((row) => row.node_id),
      nodes: sorted
    });
  }

  tags.sort((a, b) => {
    const p = Number(b.tag_priority || 0) - Number(a.tag_priority || 0);
    if (Math.abs(p) > 1e-9) return p;
    return String(a.tag).localeCompare(String(b.tag));
  });

  const nodeScores: Record<string, any> = {};
  for (const row of scored) {
    nodeScores[row.node_id] = {
      priority_score: row.priority_score,
      level: row.level,
      level_token: row.level_token,
      recency_score: row.recency_score,
      dream_score: row.dream_score,
      tags: row.tags
    };
  }

  return {
    ok: true,
    type: 'tag_memory_matrix',
    schema_version: '1.0',
    generated_at: nowIso(),
    reason,
    matrix_paths: {
      json: relPath(MATRIX_JSON_PATH),
      markdown: relPath(MATRIX_MD_PATH)
    },
    policy,
    stats: {
      nodes_scored: scored.length,
      tags_indexed: tags.length,
      dream_nodes_touched: dreamSignals.nodeHits.size,
      dream_tags_touched: dreamSignals.tagHits.size
    },
    tags,
    node_scores: nodeScores
  };
}

function buildMarkdown(payload: any) {
  const lines: string[] = [];
  lines.push('# TAG_MEMORY_MATRIX.md');
  lines.push(`# Generated: ${payload.generated_at}`);
  lines.push(`# Source: ${relPath(MEMORY_INDEX_PATH)} + dream outputs + conversation_eye runtime`);
  lines.push(`# Priority model: level(node1>tag2>jot3) + recency + dream-inclusion`);
  lines.push('');
  lines.push('## Stats');
  lines.push(`- Nodes scored: ${Number(payload && payload.stats && payload.stats.nodes_scored || 0)}`);
  lines.push(`- Tags indexed: ${Number(payload && payload.stats && payload.stats.tags_indexed || 0)}`);
  lines.push(`- Dream nodes touched: ${Number(payload && payload.stats && payload.stats.dream_nodes_touched || 0)}`);
  lines.push(`- Dream tags touched: ${Number(payload && payload.stats && payload.stats.dream_tags_touched || 0)}`);
  lines.push('');
  lines.push('## Tag Priority Order');

  const tags = Array.isArray(payload && payload.tags) ? payload.tags : [];
  for (const tagEntry of tags) {
    const tag = cleanText(tagEntry && tagEntry.tag, 80);
    lines.push(`### #${tag} (priority=${Number(tagEntry && tagEntry.tag_priority || 0)}, nodes=${Number(tagEntry && tagEntry.node_count || 0)})`);
    const nodes = Array.isArray(tagEntry && tagEntry.nodes) ? tagEntry.nodes : [];
    for (const node of nodes) {
      lines.push(`- ${node.node_id} (${node.hex_id}) score=${node.priority_score} level=${node.level_token} date=${node.date || 'n/a'} dream_hits=${Number(node.dream_node_hits || 0)}`);
    }
    lines.push('');
  }

  return `${lines.join('\n')}\n`;
}

function writeMatrixArtifacts(payload: any) {
  ensureDir(path.dirname(MATRIX_JSON_PATH));
  ensureDir(path.dirname(MATRIX_MD_PATH));
  fs.writeFileSync(MATRIX_JSON_PATH, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
  fs.writeFileSync(MATRIX_MD_PATH, buildMarkdown(payload), 'utf8');
}

function status() {
  const payload = readJsonSafe(MATRIX_JSON_PATH, null);
  if (!payload || typeof payload !== 'object') {
    return {
      ok: false,
      type: 'tag_memory_matrix_status',
      reason: 'missing_matrix',
      matrix_path: relPath(MATRIX_JSON_PATH)
    };
  }
  const tags = Array.isArray(payload.tags) ? payload.tags : [];
  return {
    ok: true,
    type: 'tag_memory_matrix_status',
    matrix_path: relPath(MATRIX_JSON_PATH),
    markdown_path: relPath(MATRIX_MD_PATH),
    generated_at: payload.generated_at || null,
    tags_indexed: tags.length,
    top_tags: tags.slice(0, 8).map((row: any) => ({
      tag: row.tag,
      tag_priority: row.tag_priority,
      node_count: row.node_count
    }))
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

function buildTagMemoryMatrix(opts: any = {}) {
  const payload = buildMatrixPayload(opts);
  const apply = opts && typeof opts.apply === 'boolean' ? opts.apply : true;
  if (payload && payload.ok === true && payload.skipped !== true && apply === true) {
    writeMatrixArtifacts(payload);
  }
  return {
    ...payload,
    applied: payload && payload.ok === true && payload.skipped !== true ? apply : false
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'run', 32).toLowerCase() || 'run';
  if (cmd === 'status') {
    process.stdout.write(`${JSON.stringify(status(), null, 2)}\n`);
    process.exit(0);
  }
  if (cmd === 'run' || cmd === 'build') {
    const apply = parseBool(args.apply, true);
    const reason = cleanText(args.reason || 'manual', 120) || 'manual';
    const out = buildTagMemoryMatrix({ apply, reason });
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
  MATRIX_JSON_PATH,
  MATRIX_MD_PATH,
  buildTagMemoryMatrix,
  buildMatrixPayload,
  status
};
