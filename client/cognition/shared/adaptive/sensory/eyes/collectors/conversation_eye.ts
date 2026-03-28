/**
 * adaptive/sensory/eyes/collectors/conversation_eye.ts
 *
 * Thin wrapper over Rust-authoritative conversation-eye collector kernel.
 * Client side keeps only callback orchestration (synthesizer + memory recall).
 */

const fs = require('fs');
const path = require('path');
const { createOpsLaneBridge } = require('../../../../../../runtime/lib/rust_lane_bridge.js');
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

function resolveWorkspaceRoot(startDir = __dirname) {
  let dir = path.resolve(startDir);
  while (true) {
    const marker = path.join(dir, 'core', 'layer0', 'ops', 'Cargo.toml');
    if (fs.existsSync(marker)) return dir;
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return path.resolve(startDir, '../../../../../../..');
}

const WORKSPACE_ROOT = resolveWorkspaceRoot();
const DEFAULT_HISTORY_PATH = process.env.CONVERSATION_EYE_HISTORY_PATH
  ? path.resolve(process.env.CONVERSATION_EYE_HISTORY_PATH)
  : path.join(WORKSPACE_ROOT, 'local', 'state', 'cockpit', 'inbox', 'history.jsonl');
const DEFAULT_LATEST_PATH = process.env.CONVERSATION_EYE_LATEST_PATH
  ? path.resolve(process.env.CONVERSATION_EYE_LATEST_PATH)
  : path.join(WORKSPACE_ROOT, 'local', 'state', 'cockpit', 'inbox', 'latest.json');
const CONVERSATION_MEMORY_DIR = process.env.CONVERSATION_EYE_MEMORY_DIR
  ? path.resolve(process.env.CONVERSATION_EYE_MEMORY_DIR)
  : path.join(WORKSPACE_ROOT, 'local', 'state', 'memory', 'conversation_eye');
const CONVERSATION_MEMORY_JSONL = path.join(CONVERSATION_MEMORY_DIR, 'nodes.jsonl');
const CONVERSATION_MEMORY_INDEX = path.join(CONVERSATION_MEMORY_DIR, 'index.json');

const WEEKLY_NODE_LIMIT = Math.max(1, Math.min(50, Number(process.env.CONVERSATION_EYE_WEEKLY_NODE_LIMIT || 10) || 10));
const WEEKLY_PROMOTION_OVERRIDES = Math.max(0, Math.min(20, Number(process.env.CONVERSATION_EYE_WEEKLY_PROMOTION_OVERRIDES || 2) || 2));
const CONVERSATION_EYE_MAX_ITEMS_CAP = Math.max(1, Math.min(32, Number(process.env.CONVERSATION_EYE_MAX_ITEMS || 3) || 3));
const CONVERSATION_EYE_MAX_ROWS_CAP = Math.max(4, Math.min(256, Number(process.env.CONVERSATION_EYE_MAX_ROWS || 24) || 24));
const CONVERSATION_EYE_MAX_WORK_MS = Math.max(1000, Math.min(30000, Number(process.env.CONVERSATION_EYE_MAX_WORK_MS || 7000) || 7000));

process.env.PROTHEUS_OPS_USE_PREBUILT = '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';

const conversationEyeBridge = createOpsLaneBridge(
  __dirname,
  'conversation_eye_collector',
  'conversation-eye-collector-kernel',
  { preferLocalCore: true }
);

function nowIso() {
  return new Date().toISOString();
}

function cleanText(v, maxLen = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function clampInt(v, min, max, fallback) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  return Math.max(min, Math.min(max, Math.floor(n)));
}

function invokeKernel(command, payload = {}, requireOk = true) {
  const encoded = Buffer.from(JSON.stringify(payload), 'utf8').toString('base64');
  const out = conversationEyeBridge.run([command, `--payload-base64=${encoded}`]);
  const status = Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1;
  if (status !== 0) {
    const detail = cleanText(
      (out && out.stderr) ||
      (out && out.stdout) ||
      (out && out.payload && out.payload.error) ||
      '',
      220
    );
    throw new Error(detail || `conversation_eye_collector_kernel_${command}_failed`);
  }

  const payloadOut = out && out.payload && out.payload.payload && typeof out.payload.payload === 'object'
    ? out.payload.payload
    : null;
  if (!payloadOut || (requireOk && payloadOut.ok !== true)) {
    throw new Error(`conversation_eye_collector_kernel_${command}_invalid_payload`);
  }
  return payloadOut;
}

function preflightConversationEye(eyeConfig, budgets) {
  return invokeKernel('preflight', {
    max_items: Number(budgets && budgets.max_items) || 0,
    history_path: DEFAULT_HISTORY_PATH,
    latest_path: DEFAULT_LATEST_PATH,
    eye_id: cleanText(eyeConfig && eyeConfig.id, 80) || 'conversation_eye',
  }, false);
}

async function collectConversationEye(eyeConfig, budgets) {
  const started = Date.now();
  const begin = invokeKernel('begin-collection', {
    eye_config: eyeConfig && typeof eyeConfig === 'object' ? eyeConfig : {},
    budgets: budgets && typeof budgets === 'object' ? budgets : {},
    history_path: DEFAULT_HISTORY_PATH,
    latest_path: DEFAULT_LATEST_PATH,
    memory_jsonl_path: CONVERSATION_MEMORY_JSONL,
    index_path: CONVERSATION_MEMORY_INDEX,
    max_items: CONVERSATION_EYE_MAX_ITEMS_CAP,
    max_rows: CONVERSATION_EYE_MAX_ROWS_CAP,
    max_work_ms: CONVERSATION_EYE_MAX_WORK_MS,
    weekly_node_limit: WEEKLY_NODE_LIMIT,
    weekly_promotion_overrides: WEEKLY_PROMOTION_OVERRIDES,
    eye_id: cleanText(eyeConfig && eyeConfig.id, 80) || 'conversation_eye',
  });

  if (!begin || begin.ok !== true) {
    const first = (begin && begin.preflight && Array.isArray(begin.preflight.failures) ? begin.preflight.failures[0] : null) || {};
    const err = new Error(`conversation_eye_preflight_failed (${cleanText(first.message || 'unknown', 160)})`);
    err.code = String(first.code || 'conversation_eye_preflight_failed');
    throw err;
  }

  const maxItems = clampInt(begin.max_items, 1, CONVERSATION_EYE_MAX_ITEMS_CAP, CONVERSATION_EYE_MAX_ITEMS_CAP);
  const maxWorkMs = clampInt(begin.max_work_ms, 1000, CONVERSATION_EYE_MAX_WORK_MS, CONVERSATION_EYE_MAX_WORK_MS);
  const weeklyNodeLimit = clampInt(begin.weekly_node_limit, 1, WEEKLY_NODE_LIMIT, WEEKLY_NODE_LIMIT);
  const weeklyPromotionOverrides = clampInt(
    begin.weekly_promotion_overrides,
    0,
    WEEKLY_PROMOTION_OVERRIDES,
    WEEKLY_PROMOTION_OVERRIDES
  );
  const topics = Array.isArray(begin.topics) ? begin.topics : [];
  const sourceRows = Array.isArray(begin.source_rows) ? begin.source_rows : [];
  const index = begin.index && typeof begin.index === 'object'
    ? begin.index
    : { version: '1.0', emitted_node_ids: {}, weekly_counts: {}, weekly_promotions: {} };

  const candidates = [];
  let workBudgetExceeded = false;

  for (let i = sourceRows.length - 1; i >= 0; i -= 1) {
    if ((Date.now() - started) >= maxWorkMs) {
      workBudgetExceeded = true;
      break;
    }

    const row = sourceRows[i];
    const node = synthesizeEnvelope(row);
    if (!node || !node.node_id) continue;

    const nodeTags = Array.isArray(node.node_tags)
      ? node.node_tags
      : ['conversation', 'decision', 'insight', 'directive', 't1'];
    const recall = await processMemoryFiled(
      {
        node_id: node.node_id,
        tags: nodeTags,
        source: 'conversation_eye'
      },
      {
        dryRun: String(process.env.CONVERSATION_EYE_AUTO_RECALL_DRY_RUN || '0').trim() === '1'
      }
    );

    candidates.push({
      node,
      recall: recall && typeof recall === 'object' ? recall : null
    });
  }

  const processed = invokeKernel('process-nodes', {
    index,
    candidates,
    topics,
    weekly_node_limit: weeklyNodeLimit,
    weekly_promotion_overrides: weeklyPromotionOverrides,
    max_items: maxItems,
    now_ts: nowIso(),
  });

  const memoryRows = Array.isArray(processed.memory_rows) ? processed.memory_rows : [];
  if (memoryRows.length > 0) {
    invokeKernel('append-memory-rows', {
      jsonl_path: cleanText(begin.memory_jsonl_path, 600) || CONVERSATION_MEMORY_JSONL,
      rows: memoryRows
    });
  }

  invokeKernel('save-index', {
    index_path: cleanText(begin.index_path, 600) || CONVERSATION_MEMORY_INDEX,
    index: processed.index && typeof processed.index === 'object' ? processed.index : index,
  });

  const items = Array.isArray(processed.items) ? processed.items : [];
  const nodeWrites = Number.isFinite(Number(processed.node_writes)) ? Number(processed.node_writes) : 0;
  const recallQueued = Number.isFinite(Number(processed.recall_queued)) ? Number(processed.recall_queued) : 0;
  const recallMatched = Number.isFinite(Number(processed.recall_matches)) ? Number(processed.recall_matches) : 0;
  const quotaSkipped = Number.isFinite(Number(processed.quota_skipped)) ? Number(processed.quota_skipped) : 0;

  return {
    success: true,
    items,
    duration_ms: Date.now() - started,
    requests: 0,
    bytes: items.reduce((sum, item) => sum + Number((item && item.bytes) || 0), 0),
    metadata: {
      node_writes: nodeWrites,
      source_rows_seen: sourceRows.length,
      recall_queued: recallQueued,
      recall_matches: recallMatched,
      quota_skipped: quotaSkipped,
      work_budget_exceeded: workBudgetExceeded,
      max_work_ms: maxWorkMs,
      max_rows_cap: clampInt(begin.max_rows, 4, CONVERSATION_EYE_MAX_ROWS_CAP, CONVERSATION_EYE_MAX_ROWS_CAP),
      max_items_cap: maxItems,
    }
  };
}

module.exports = {
  collectConversationEye,
  preflightConversationEye
};
