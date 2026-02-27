#!/usr/bin/env node
'use strict';
export {};

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');

type AnyObj = Record<string, any>;

const ROOT = path.resolve(__dirname, '..', '..');
const DEFAULT_LOG_DIR = process.env.CANONICAL_EVENT_LOG_DIR
  ? path.resolve(process.env.CANONICAL_EVENT_LOG_DIR)
  : path.join(ROOT, 'state', 'runtime', 'canonical_events');

function nowIso() {
  return new Date().toISOString();
}

function cleanText(v: unknown, maxLen = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function stableStringify(v: unknown): string {
  if (v == null || typeof v !== 'object') return JSON.stringify(v);
  if (Array.isArray(v)) return `[${v.map((row) => stableStringify(row)).join(',')}]`;
  const obj = v as AnyObj;
  const keys = Object.keys(obj).sort((a, b) => a.localeCompare(b));
  const fields = keys.map((k) => `${JSON.stringify(k)}:${stableStringify(obj[k])}`);
  return `{${fields.join(',')}}`;
}

function readJsonl(filePath: string) {
  try {
    if (!fs.existsSync(filePath)) return [];
    return fs.readFileSync(filePath, 'utf8')
      .split('\n')
      .filter(Boolean)
      .map((line) => {
        try { return JSON.parse(line); } catch { return null; }
      })
      .filter(Boolean);
  } catch {
    return [];
  }
}

function appendJsonl(filePath: string, row: AnyObj) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`, 'utf8');
}

function defaultLogPathForTs(tsIso: string) {
  const day = cleanText(tsIso || nowIso(), 10) || nowIso().slice(0, 10);
  return path.join(DEFAULT_LOG_DIR, `${day}.jsonl`);
}

function resolveLogPath(opts: AnyObj = {}) {
  if (opts.path) {
    return path.isAbsolute(opts.path) ? opts.path : path.join(ROOT, String(opts.path));
  }
  if (opts.date) {
    const date = cleanText(opts.date, 10) || nowIso().slice(0, 10);
    return path.join(DEFAULT_LOG_DIR, `${date}.jsonl`);
  }
  return defaultLogPathForTs(opts.ts || nowIso());
}

function readLastRow(filePath: string) {
  const rows = readJsonl(filePath);
  return rows.length ? rows[rows.length - 1] : null;
}

function hashCanonicalRow(rowNoHash: AnyObj) {
  return crypto.createHash('sha256').update(stableStringify(rowNoHash)).digest('hex');
}

function appendCanonicalEvent(eventRaw: AnyObj, opts: AnyObj = {}) {
  const ts = cleanText(eventRaw && eventRaw.ts ? eventRaw.ts : nowIso(), 40) || nowIso();
  const logPath = resolveLogPath({ ...opts, ts });
  const prev = readLastRow(logPath);
  const prevHash = prev && typeof prev.hash === 'string' ? String(prev.hash) : null;
  const prevSeq = Number(prev && prev.seq || 0);
  const eventId = cleanText(eventRaw && eventRaw.event_id ? eventRaw.event_id : '', 80)
    || `evt_${crypto.createHash('sha1').update(`${ts}|${Math.random()}`).digest('hex').slice(0, 12)}`;
  const rowNoHash = {
    schema_id: 'canonical_runtime_event',
    schema_version: '1.0',
    ts,
    date: ts.slice(0, 10),
    seq: Number.isFinite(prevSeq) ? prevSeq + 1 : 1,
    event_id: eventId,
    prev_hash: prevHash,
    type: cleanText(eventRaw && eventRaw.type ? eventRaw.type : 'runtime_event', 80) || 'runtime_event',
    phase: cleanText(eventRaw && eventRaw.phase ? eventRaw.phase : '', 40) || null,
    run_id: cleanText(eventRaw && eventRaw.run_id ? eventRaw.run_id : '', 120) || null,
    workflow_id: cleanText(eventRaw && eventRaw.workflow_id ? eventRaw.workflow_id : '', 120) || null,
    step_id: cleanText(eventRaw && eventRaw.step_id ? eventRaw.step_id : '', 120) || null,
    opcode: cleanText(eventRaw && eventRaw.opcode ? eventRaw.opcode : '', 80).toUpperCase() || null,
    effect: cleanText(eventRaw && eventRaw.effect ? eventRaw.effect : '', 80).toLowerCase() || null,
    ok: eventRaw && typeof eventRaw.ok === 'boolean' ? eventRaw.ok : null,
    payload: eventRaw && eventRaw.payload && typeof eventRaw.payload === 'object' ? eventRaw.payload : {}
  };
  const row = {
    ...rowNoHash,
    hash: hashCanonicalRow(rowNoHash)
  };
  appendJsonl(logPath, row);
  const latestPath = path.join(DEFAULT_LOG_DIR, 'latest.json');
  try {
    fs.mkdirSync(path.dirname(latestPath), { recursive: true });
    fs.writeFileSync(latestPath, `${JSON.stringify({
      ts,
      log_path: path.relative(ROOT, logPath),
      event_id: row.event_id,
      seq: row.seq,
      hash: row.hash
    }, null, 2)}\n`, 'utf8');
  } catch {
    // Best effort.
  }
  return {
    ...row,
    log_path: logPath
  };
}

function verifyCanonicalEvents(targetRaw: unknown) {
  const target = cleanText(targetRaw || '', 400) || DEFAULT_LOG_DIR;
  const absTarget = path.isAbsolute(target) ? target : path.join(ROOT, target);
  let files: string[] = [];
  if (fs.existsSync(absTarget) && fs.statSync(absTarget).isDirectory()) {
    files = fs.readdirSync(absTarget)
      .filter((name) => name.endsWith('.jsonl'))
      .map((name) => path.join(absTarget, name))
      .sort((a, b) => a.localeCompare(b));
  } else if (fs.existsSync(absTarget)) {
    files = [absTarget];
  }
  const failures: AnyObj[] = [];
  let totalEvents = 0;
  let lastHash = null;
  for (const filePath of files) {
    const rows = readJsonl(filePath);
    let expectedPrev = null;
    for (const row of rows) {
      totalEvents += 1;
      const rowNoHash = { ...row };
      delete rowNoHash.hash;
      const recomputed = hashCanonicalRow(rowNoHash);
      if (String(row.hash || '') !== recomputed) {
        failures.push({
          type: 'hash_mismatch',
          file: path.relative(ROOT, filePath),
          seq: Number(row.seq || 0),
          event_id: row.event_id || null
        });
      }
      if ((row.prev_hash || null) !== expectedPrev) {
        failures.push({
          type: 'prev_hash_mismatch',
          file: path.relative(ROOT, filePath),
          seq: Number(row.seq || 0),
          event_id: row.event_id || null
        });
      }
      expectedPrev = row.hash || null;
      lastHash = row.hash || lastHash;
    }
  }
  return {
    ok: failures.length === 0,
    checked_files: files.map((fp) => path.relative(ROOT, fp)),
    total_events: totalEvents,
    last_hash: lastHash,
    failures
  };
}

module.exports = {
  DEFAULT_LOG_DIR,
  appendCanonicalEvent,
  verifyCanonicalEvents
};
