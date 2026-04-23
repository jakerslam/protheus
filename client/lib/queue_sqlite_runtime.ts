#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const path = require('path');
const { createOpsLaneBridge } = require('../runtime/lib/rust_lane_bridge.ts');

process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'queue_sqlite_runtime', 'queue-sqlite-kernel');

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function text(value, maxLen = 320) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function normalizeSqliteCfg(input) {
  const raw = input && input.__queue_sqlite_kernel && input.sqlite_cfg
    ? input.sqlite_cfg
    : (input && typeof input === 'object' ? input : {});
  const dbPath = text(raw.db_path, 520);
  if (!dbPath) throw new Error('queue_sqlite_db_path_required');
  return {
    db_path: path.resolve(dbPath),
    journal_mode: text(raw.journal_mode || 'WAL', 24).toUpperCase() || 'WAL',
    synchronous: text(raw.synchronous || 'NORMAL', 24).toUpperCase() || 'NORMAL',
    busy_timeout_ms: Number.isFinite(Number(raw.busy_timeout_ms))
      ? Math.max(100, Math.min(120000, Math.floor(Number(raw.busy_timeout_ms))))
      : 5000
  };
}

function sqliteToken(input) {
  return {
    __queue_sqlite_kernel: true,
    sqlite_cfg: normalizeSqliteCfg(input)
  };
}

function invoke(command, payload = {}, opts = {}) {
  const out = bridge.run([
    command,
    `--payload-base64=${encodeBase64(JSON.stringify(payload || {}))}`
  ]);
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  const payloadOut = receipt && receipt.payload && typeof receipt.payload === 'object'
    ? receipt.payload
    : receipt;
  if (out.status !== 0) {
    const message = payloadOut && typeof payloadOut.error === 'string'
      ? payloadOut.error
      : (out && out.stderr ? String(out.stderr).trim() : `queue_sqlite_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `queue_sqlite_kernel_${command}_failed`);
    return { ok: false, error: message || `queue_sqlite_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `queue_sqlite_kernel_${command}_bridge_failed`
      : `queue_sqlite_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function openQueueDb(sqliteCfg) {
  const token = sqliteToken(sqliteCfg);
  invoke('open', { sqlite_cfg: token.sqlite_cfg });
  return token;
}

function ensureQueueSchema(db) {
  const token = sqliteToken(db);
  invoke('ensure-schema', { sqlite_cfg: token.sqlite_cfg });
  return { ok: true, sqlite_cfg: token.sqlite_cfg };
}

function migrateHistoryJsonl(db, historyPath, queueName = 'backlog_queue_executor') {
  const token = sqliteToken(db);
  return invoke('migrate-history', {
    sqlite_cfg: token.sqlite_cfg,
    history_path: path.resolve(String(historyPath || '')),
    queue_name: String(queueName || 'backlog_queue_executor')
  });
}

function upsertQueueItem(db, queueName, row, status) {
  const token = sqliteToken(db);
  return invoke('upsert-item', {
    sqlite_cfg: token.sqlite_cfg,
    queue_name: String(queueName || 'default_queue'),
    row: row && typeof row === 'object' ? row : {},
    status: String(status || (row && row.status) || 'queued')
  });
}

function appendQueueEvent(db, queueName, laneId, eventType, payload, ts = null) {
  const token = sqliteToken(db);
  return invoke('append-event', {
    sqlite_cfg: token.sqlite_cfg,
    queue_name: String(queueName || 'default_queue'),
    lane_id: String(laneId || ''),
    event_type: String(eventType || 'event'),
    payload: payload && typeof payload === 'object' ? payload : {},
    ts: ts == null ? undefined : String(ts)
  });
}

function insertReceipt(db, laneId, receipt) {
  const token = sqliteToken(db);
  return invoke('insert-receipt', {
    sqlite_cfg: token.sqlite_cfg,
    lane_id: String(laneId || ''),
    receipt: receipt && typeof receipt === 'object' ? receipt : {}
  });
}

function queueStats(db, queueName) {
  const token = sqliteToken(db);
  return invoke('queue-stats', {
    sqlite_cfg: token.sqlite_cfg,
    queue_name: String(queueName || 'default_queue')
  });
}

module.exports = {
  openQueueDb,
  ensureQueueSchema,
  migrateHistoryJsonl,
  upsertQueueItem,
  appendQueueEvent,
  insertReceipt,
  queueStats
};
