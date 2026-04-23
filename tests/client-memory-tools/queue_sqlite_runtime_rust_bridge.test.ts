#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../..');

function resetModule(modulePath) {
  delete require.cache[require.resolve(modulePath)];
  return require(modulePath);
}

function main() {
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), 'queue-sqlite-rust-'));
  const dbPath = path.join(workspace, 'state', 'queue.sqlite');
  const historyPath = path.join(workspace, 'history.jsonl');
  fs.mkdirSync(path.dirname(dbPath), { recursive: true });
  fs.writeFileSync(
    historyPath,
    `${JSON.stringify({ lane_id: 'BL-1', action: 'queued', ts: '2026-03-17T00:00:00Z' })}\n`
  );

  const prevUsePrebuilt = process.env.INFRING_OPS_USE_PREBUILT;
  const prevTimeout = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS;
  process.env.INFRING_OPS_USE_PREBUILT = '0';
  process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = '120000';

  try {
    const mod = resetModule(path.join(ROOT, 'client/lib/queue_sqlite_runtime.ts'));
    const db = mod.openQueueDb({ db_path: dbPath, journal_mode: 'wal', synchronous: 'normal' });
    assert.equal(db.__queue_sqlite_kernel, true);
    assert.equal(db.sqlite_cfg.db_path, dbPath);
    assert.equal(typeof db.sqlite_cfg.journal_mode, 'string');

    const schema = mod.ensureQueueSchema(db);
    assert.equal(schema.ok, true);
    assert.equal(fs.existsSync(dbPath), true);

    const migrated = mod.migrateHistoryJsonl(db, historyPath, 'backlog_queue_executor');
    assert.equal(migrated.ok, true);
    assert.equal(migrated.rows_migrated, 1);

    const upserted = mod.upsertQueueItem(db, 'backlog_queue_executor', {
      id: 'BL-1',
      class: 'memory',
      wave: 'w1',
      title: 'Ship queue kernel',
      dependencies: ['BL-0']
    }, 'queued');
    assert.equal(upserted.ok, true);
    assert.equal(upserted.lane_id, 'BL-1');

    const event = mod.appendQueueEvent(db, 'backlog_queue_executor', 'BL-1', 'started', { detail: 'running' });
    assert.equal(event.ok, true);
    assert.ok(event.event_id);

    const receipt = mod.insertReceipt(db, 'BL-1', { ok: true, ts: '2026-03-17T00:01:00Z' });
    assert.equal(receipt.ok, true);
    assert.ok(receipt.receipt_id);

    const stats = mod.queueStats(db, 'backlog_queue_executor');
    assert.equal(stats.ok, true);
    assert.equal(stats.items, 1);
    assert.equal(stats.events, 2);
    assert.equal(stats.receipts, 1);
    assert.equal(Number.isFinite(Number(stats.items)), true);
    assert.ok(['number', 'string'].includes(typeof stats.receipts));
  } finally {
    if (prevUsePrebuilt == null) delete process.env.INFRING_OPS_USE_PREBUILT;
    else process.env.INFRING_OPS_USE_PREBUILT = prevUsePrebuilt;
    if (prevTimeout == null) delete process.env.INFRING_OPS_LOCAL_TIMEOUT_MS;
    else process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = prevTimeout;
  }

  console.log(JSON.stringify({ ok: true, type: 'queue_sqlite_runtime_rust_bridge_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
