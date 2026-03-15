/**
 * adaptive/sensory/eyes/collectors/conversation_eye.ts
 *
 * Conversation Eye
 * - Ingests cockpit envelope history (push context).
 * - Synthesizes dialogue/decision insights into tagged memory nodes.
 * - Emits external_eyes-compatible signal items.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { synthesizeEnvelope } = require('../../../../../../runtime/systems/sensory/conversation_eye_synthesizer.ts');

let processMemoryFiled = async () => ({ matches: [], attention: { queued: false } });
{
  const recallCandidates = [
    '../../../../../runtime/systems/memory/memory_auto_recall.ts',
    '../../../../../runtime/systems/memory/memory_recall.ts'
  ];
  for (const candidate of recallCandidates) {
    const resolved = path.resolve(__dirname, candidate);
    if (!fs.existsSync(resolved)) continue;
    try {
      const mod = require(resolved);
      if (mod && typeof mod.processMemoryFiled === 'function') {
        processMemoryFiled = mod.processMemoryFiled;
        break;
      }
      if (mod && typeof mod.processMemoryFile === 'function') {
        processMemoryFiled = mod.processMemoryFile;
        break;
      }
    } catch {
      // fail-soft: conversation eye still emits nodes without recall augmentation.
    }
  }
}

const WORKSPACE_DIR = path.join(__dirname, '..', '..', '..', '..');
const DEFAULT_HISTORY_PATH = process.env.CONVERSATION_EYE_HISTORY_PATH
  ? path.resolve(process.env.CONVERSATION_EYE_HISTORY_PATH)
  : path.join(WORKSPACE_DIR, 'local', 'state', 'cockpit', 'inbox', 'history.jsonl');
const DEFAULT_LATEST_PATH = process.env.CONVERSATION_EYE_LATEST_PATH
  ? path.resolve(process.env.CONVERSATION_EYE_LATEST_PATH)
  : path.join(WORKSPACE_DIR, 'local', 'state', 'cockpit', 'inbox', 'latest.json');
const CONVERSATION_MEMORY_DIR = process.env.CONVERSATION_EYE_MEMORY_DIR
  ? path.resolve(process.env.CONVERSATION_EYE_MEMORY_DIR)
  : path.join(WORKSPACE_DIR, 'local', 'state', 'memory', 'conversation_eye');
const CONVERSATION_MEMORY_JSONL = path.join(CONVERSATION_MEMORY_DIR, 'nodes.jsonl');
const CONVERSATION_MEMORY_INDEX = path.join(CONVERSATION_MEMORY_DIR, 'index.json');
const WEEKLY_NODE_LIMIT = Math.max(1, Math.min(50, Number(process.env.CONVERSATION_EYE_WEEKLY_NODE_LIMIT || 10) || 10));
const WEEKLY_PROMOTION_OVERRIDES = Math.max(0, Math.min(20, Number(process.env.CONVERSATION_EYE_WEEKLY_PROMOTION_OVERRIDES || 2) || 2));
const CONVERSATION_EYE_MAX_ITEMS_CAP = Math.max(1, Math.min(32, Number(process.env.CONVERSATION_EYE_MAX_ITEMS || 3) || 3));
const CONVERSATION_EYE_MAX_ROWS_CAP = Math.max(4, Math.min(256, Number(process.env.CONVERSATION_EYE_MAX_ROWS || 24) || 24));
const CONVERSATION_EYE_MAX_WORK_MS = Math.max(1000, Math.min(30000, Number(process.env.CONVERSATION_EYE_MAX_WORK_MS || 7000) || 7000));

function nowIso() {
  return new Date().toISOString();
}

function cleanText(v, maxLen = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function sha16(v) {
  return crypto.createHash('sha256').update(String(v == null ? '' : v), 'utf8').digest('hex').slice(0, 16);
}

function ensureDir(absDir) {
  fs.mkdirSync(absDir, { recursive: true });
}

function readJsonSafe(filePath, fallback = null) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function readJsonlTail(filePath, maxLines = 64) {
  if (!fs.existsSync(filePath)) return [];
  const raw = fs.readFileSync(filePath, 'utf8');
  const lines = raw.split('\n').map((line) => line.trim()).filter(Boolean);
  const tail = lines.slice(Math.max(0, lines.length - Math.max(1, Number(maxLines) || 64)));
  const out = [];
  for (const line of tail) {
    try {
      out.push(JSON.parse(line));
    } catch {
      // malformed lines are skipped; collector remains fail-soft.
    }
  }
  return out;
}

function loadMemoryIndex() {
  const base = readJsonSafe(CONVERSATION_MEMORY_INDEX, {
    version: '1.0',
    updated_ts: null,
    emitted_node_ids: {},
    weekly_counts: {},
    weekly_promotions: {}
  });
  if (!base || typeof base !== 'object') {
    return { version: '1.0', updated_ts: null, emitted_node_ids: {}, weekly_counts: {}, weekly_promotions: {} };
  }
  if (!base.emitted_node_ids || typeof base.emitted_node_ids !== 'object') {
    base.emitted_node_ids = {};
  }
  if (!base.weekly_counts || typeof base.weekly_counts !== 'object') {
    base.weekly_counts = {};
  }
  if (!base.weekly_promotions || typeof base.weekly_promotions !== 'object') {
    base.weekly_promotions = {};
  }
  return base;
}

function saveMemoryIndex(index) {
  ensureDir(CONVERSATION_MEMORY_DIR);
  const out = {
    version: '1.0',
    updated_ts: nowIso(),
    emitted_node_ids: index && typeof index.emitted_node_ids === 'object'
      ? index.emitted_node_ids
      : {},
    weekly_counts: index && typeof index.weekly_counts === 'object'
      ? index.weekly_counts
      : {},
    weekly_promotions: index && typeof index.weekly_promotions === 'object'
      ? index.weekly_promotions
      : {}
  };
  fs.writeFileSync(CONVERSATION_MEMORY_INDEX, `${JSON.stringify(out, null, 2)}\n`, 'utf8');
}

function appendMemoryNode(row) {
  ensureDir(CONVERSATION_MEMORY_DIR);
  fs.appendFileSync(CONVERSATION_MEMORY_JSONL, `${JSON.stringify(row)}\n`, 'utf8');
}

function normalizeTopics(eyeConfig) {
  const defaults = ['conversation', 'decision', 'insight', 'directive', 't1'];
  const topics = Array.isArray(eyeConfig && eyeConfig.topics) ? eyeConfig.topics : [];
  const out = [];
  for (const raw of defaults.concat(topics)) {
    const value = cleanText(raw, 48).toLowerCase();
    if (!value) continue;
    if (!out.includes(value)) out.push(value);
  }
  return out.slice(0, 8);
}

function toIsoWeek(ts) {
  const date = new Date(String(ts || nowIso()));
  if (Number.isNaN(date.getTime())) return 'unknown-week';
  const d = new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate()));
  const day = d.getUTCDay() || 7;
  d.setUTCDate(d.getUTCDate() + 4 - day);
  const yearStart = new Date(Date.UTC(d.getUTCFullYear(), 0, 1));
  const weekNo = Math.ceil((((d.getTime() - yearStart.getTime()) / 86400000) + 1) / 7);
  return `${d.getUTCFullYear()}-W${String(weekNo).padStart(2, '0')}`;
}

function canWriteNodeForWeek(index, node) {
  const weekKey = toIsoWeek(node && node.ts);
  const count = Number(index && index.weekly_counts && index.weekly_counts[weekKey] || 0);
  const promotions = Number(index && index.weekly_promotions && index.weekly_promotions[weekKey] || 0);
  const level = Math.max(1, Math.min(3, Number(node && node.level || 3) || 3));
  if (count < WEEKLY_NODE_LIMIT) {
    return { allowed: true, weekKey, promoted: false, count, promotions };
  }
  if (level === 1 && promotions < WEEKLY_PROMOTION_OVERRIDES) {
    return { allowed: true, weekKey, promoted: true, count, promotions };
  }
  return { allowed: false, weekKey, promoted: false, count, promotions };
}

function synthesizeFromSource(maxRows) {
  const historyRows = readJsonlTail(DEFAULT_HISTORY_PATH, maxRows);
  if (historyRows.length > 0) return historyRows;
  const latest = readJsonSafe(DEFAULT_LATEST_PATH, null);
  return latest && typeof latest === 'object' ? [latest] : [];
}

function toCollectItem(node, topics, recall = null) {
  const date = cleanText(node && node.date, 20) || nowIso().slice(0, 10);
  const nodeId = cleanText(node && node.node_id, 80) || `conversation-eye-${sha16(`${date}|fallback`)}`;
  const url = `https://local.workspace/conversation/${date}/${nodeId}`;
  const title = cleanText(node && node.title, 180) || '[Conversation Eye] synthesized signal';
  const preview = cleanText(node && node.preview, 240) || 'conversation_eye synthesized runtime node';
  return {
    collected_at: nowIso(),
    id: sha16(`${nodeId}|${title}`),
    url,
    title,
    content_preview: preview,
    topics,
    node_id: nodeId,
    node_hex_id: cleanText(node && node.hex_id, 24) || null,
    node_kind: cleanText(node && node.node_kind, 32) || 'insight',
    node_level: Math.max(1, Math.min(3, Number(node && node.level || 3) || 3)),
    node_level_token: cleanText(node && node.level_token, 16) || 'jot3',
    node_tags: Array.isArray(node && node.node_tags) ? node.node_tags.slice(0, 12) : ['conversation', 'decision', 'insight', 'directive', 't1'],
    edges_to: Array.isArray(node && node.edges_to) ? node.edges_to.slice(0, 12) : [],
    recall_matches: recall && Array.isArray(recall.matches) ? recall.matches.slice(0, 3).map((row) => ({
      node_id: cleanText(row && row.node_id, 120),
      score: Number(row && row.score || 0),
      shared_tags: Array.isArray(row && row.shared_tags) ? row.shared_tags.slice(0, 8) : []
    })) : [],
    recall_queued: recall && recall.attention && recall.attention.queued === true,
    bytes: Math.min(8192, title.length + preview.length + 160)
  };
}

function preflightConversationEye(eyeConfig, budgets) {
  const checks = [];
  const failures = [];
  const maxItems = Number(budgets && budgets.max_items);
  if (!Number.isFinite(maxItems) || maxItems <= 0) {
    failures.push({ code: 'invalid_budget', message: 'budgets.max_items must be > 0' });
  } else {
    checks.push({ name: 'max_items_valid', ok: true, value: maxItems });
  }

  const historyExists = fs.existsSync(DEFAULT_HISTORY_PATH);
  const latestExists = fs.existsSync(DEFAULT_LATEST_PATH);
  if (!historyExists && !latestExists) {
    failures.push({
      code: 'conversation_source_missing',
      message: `missing cockpit context source (${DEFAULT_HISTORY_PATH} or ${DEFAULT_LATEST_PATH})`
    });
  } else {
    checks.push({
      name: 'cockpit_source_present',
      ok: true,
      history_path: DEFAULT_HISTORY_PATH,
      latest_path: DEFAULT_LATEST_PATH
    });
  }

  return {
    ok: failures.length === 0,
    parser_type: 'conversation_eye',
    checks,
    failures
  };
}

async function collectConversationEye(eyeConfig, budgets) {
  const started = Date.now();
  const preflight = preflightConversationEye(eyeConfig, budgets);
  if (!preflight.ok) {
    const first = preflight.failures[0] || {};
    const err = new Error(`conversation_eye_preflight_failed (${cleanText(first.message || 'unknown', 160)})`);
    err.code = String(first.code || 'conversation_eye_preflight_failed');
    throw err;
  }

  const maxItems = Math.max(1, Math.min(Number((budgets && budgets.max_items) || CONVERSATION_EYE_MAX_ITEMS_CAP), CONVERSATION_EYE_MAX_ITEMS_CAP));
  const maxRows = Math.max(4, Math.min(Number((budgets && budgets.max_rows) || CONVERSATION_EYE_MAX_ROWS_CAP), CONVERSATION_EYE_MAX_ROWS_CAP));
  const topics = normalizeTopics(eyeConfig);
  const sourceRows = synthesizeFromSource(maxRows);
  const index = loadMemoryIndex();
  const emitted = index.emitted_node_ids || {};
  const weeklyCounts = index.weekly_counts || {};
  const weeklyPromotions = index.weekly_promotions || {};
  const items = [];
  let nodeWrites = 0;
  let recallQueued = 0;
  let recallMatched = 0;
  let quotaSkipped = 0;
  let workBudgetExceeded = false;

  for (let i = sourceRows.length - 1; i >= 0; i -= 1) {
    if ((Date.now() - started) >= CONVERSATION_EYE_MAX_WORK_MS) {
      workBudgetExceeded = true;
      break;
    }
    const row = sourceRows[i];
    const node = synthesizeEnvelope(row);
    if (!node || !node.node_id) continue;
    if (emitted[node.node_id]) continue;
    const quota = canWriteNodeForWeek(index, node);
    if (!quota.allowed) {
      quotaSkipped += 1;
      continue;
    }
    emitted[node.node_id] = nowIso();
    weeklyCounts[quota.weekKey] = Number(weeklyCounts[quota.weekKey] || 0) + 1;
    if (quota.promoted) {
      weeklyPromotions[quota.weekKey] = Number(weeklyPromotions[quota.weekKey] || 0) + 1;
    }
    const nodeTags = Array.isArray(node.node_tags)
      ? node.node_tags
      : ['conversation', 'decision', 'insight', 'directive', 't1'];
    appendMemoryNode({
      ts: nowIso(),
      source: 'conversation_eye',
      node_id: node.node_id,
      hex_id: node.hex_id || null,
      node_kind: node.node_kind,
      level: Math.max(1, Math.min(3, Number(node.level || 3) || 3)),
      level_token: cleanText(node.level_token || 'jot3', 16),
      tags: nodeTags,
      edges_to: Array.isArray(node.edges_to) ? node.edges_to : [],
      title: node.title,
      preview: node.preview,
      xml: cleanText(node.xml, 1600) || null
    });
    const recall = await processMemoryFiled({
      node_id: node.node_id,
      tags: nodeTags,
      source: 'conversation_eye'
    }, {
      dryRun: String(process.env.CONVERSATION_EYE_AUTO_RECALL_DRY_RUN || '0').trim() === '1'
    });
    if (recall && Array.isArray(recall.matches)) recallMatched += recall.matches.length;
    if (recall && recall.attention && recall.attention.queued === true) recallQueued += 1;
    nodeWrites += 1;
    items.push(toCollectItem(node, topics, recall));
    if (items.length >= maxItems) break;
  }

  index.emitted_node_ids = emitted;
  index.weekly_counts = weeklyCounts;
  index.weekly_promotions = weeklyPromotions;
  saveMemoryIndex(index);

  return {
    success: true,
    items,
    duration_ms: Date.now() - started,
    requests: 0,
    bytes: items.reduce((sum, item) => sum + Number(item && item.bytes || 0), 0),
    metadata: {
      node_writes: nodeWrites,
      source_rows_seen: sourceRows.length,
      recall_queued: recallQueued,
      recall_matches: recallMatched,
      quota_skipped: quotaSkipped,
      work_budget_exceeded: workBudgetExceeded,
      max_work_ms: CONVERSATION_EYE_MAX_WORK_MS,
      max_rows_cap: CONVERSATION_EYE_MAX_ROWS_CAP,
      max_items_cap: CONVERSATION_EYE_MAX_ITEMS_CAP
    }
  };
}

module.exports = {
  collectConversationEye,
  preflightConversationEye
};
