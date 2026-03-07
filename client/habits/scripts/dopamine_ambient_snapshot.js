#!/usr/bin/env node
'use strict';

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');
const engine = require('./dopamine_engine.js');
const { loadMechSuitModePolicy } = require('../../lib/mech_suit_mode');

const CLIENT_ROOT = path.resolve(__dirname, '..', '..');

function isoNow() {
  return new Date().toISOString();
}

function normalizeDate(raw) {
  const value = String(raw || '').trim();
  if (/^\d{4}-\d{2}-\d{2}$/.test(value)) return value;
  return isoNow().slice(0, 10);
}

function hashObject(value) {
  return crypto.createHash('sha256').update(JSON.stringify(value)).digest('hex');
}

function readJson(filePath, fallback = null) {
  try {
    if (!filePath || !fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function appendJsonl(filePath, row) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`, 'utf8');
}

function toBool(v, fallback = false) {
  const token = String(v == null ? '' : v).trim().toLowerCase();
  if (!token) return fallback;
  if (['1', 'true', 'yes', 'on'].includes(token)) return true;
  if (['0', 'false', 'no', 'off'].includes(token)) return false;
  return fallback;
}

function policyPath() {
  const raw = String(process.env.MECH_SUIT_MODE_POLICY_PATH || '').trim();
  if (!raw) return path.join(CLIENT_ROOT, 'config', 'mech_suit_mode_policy.json');
  return path.isAbsolute(raw) ? raw : path.join(CLIENT_ROOT, raw);
}

function loadPolicy() {
  const policy = loadMechSuitModePolicy({
    root: CLIENT_ROOT,
    path: policyPath()
  });
  const dopamine = policy && policy.dopamine && typeof policy.dopamine === 'object' ? policy.dopamine : {};
  const eyes = policy && policy.eyes && typeof policy.eyes === 'object' ? policy.eyes : {};
  return {
    policy,
    threshold_breach_only: dopamine.threshold_breach_only !== false,
    attention_queue_path: String(eyes.attention_queue_path || path.join(CLIENT_ROOT, 'local', 'state', 'attention', 'queue.jsonl'))
  };
}

function closeoutSnapshot(dateStr) {
  const date = normalizeDate(dateStr);
  const captured = engine.autocap('git');
  engine.updateRollingAverages();
  engine.updateStreak(date);
  const summary = engine.getCurrentSDS();
  const out = {
    ok: true,
    type: 'dopamine_snapshot',
    mode: 'closeout',
    ts: isoNow(),
    date,
    captured,
    summary
  };
  out.receipt_hash = hashObject(out);
  return out;
}

function statusSnapshot(dateStr) {
  const date = normalizeDate(dateStr);
  const summary = engine.getCurrentSDS();
  const out = {
    ok: true,
    type: 'dopamine_snapshot',
    mode: 'status',
    ts: isoNow(),
    date,
    summary
  };
  out.receipt_hash = hashObject(out);
  return out;
}

function parseDateArg(args) {
  for (let i = 0; i < args.length; i += 1) {
    const token = String(args[i] || '').trim();
    if (token.startsWith('--date=')) return token.slice('--date='.length);
    if (token === '--date' && i + 1 < args.length) return args[i + 1];
  }
  return null;
}

function parseSummaryArg(args) {
  for (let i = 0; i < args.length; i += 1) {
    const token = String(args[i] || '').trim();
    if (token.startsWith('--summary-json=')) return token.slice('--summary-json='.length);
    if (token === '--summary-json' && i + 1 < args.length) return args[i + 1];
  }
  return null;
}

function parseSummary(summaryRaw) {
  const raw = String(summaryRaw || '').trim();
  if (!raw) return {};
  try {
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === 'object' ? parsed : {};
  } catch {
    return {};
  }
}

function evaluateBreach(summary) {
  const reasons = [];
  const sds = Number(summary && summary.sds);
  const drift = Number(summary && summary.drift_minutes);
  const switches = Number(summary && summary.context_switches);
  const painActive = !!(summary && summary.directive_pain && summary.directive_pain.active === true);
  if (Number.isFinite(sds) && sds < 0) reasons.push('sds_negative');
  if (Number.isFinite(drift) && drift >= 120) reasons.push('drift_minutes_high');
  if (Number.isFinite(switches) && switches >= 8) reasons.push('context_switches_high');
  if (painActive) reasons.push('directive_pain_active');
  return {
    breached: reasons.length > 0,
    reasons
  };
}

async function evaluateSnapshot(dateStr, summaryRaw) {
  const date = normalizeDate(dateStr);
  const policy = loadPolicy();
  const summary = parseSummary(summaryRaw);
  const breach = evaluateBreach(summary);
  const surfaced = policy.threshold_breach_only ? breach.breached : true;

  const out = {
    ok: true,
    type: 'dopamine_snapshot',
    mode: 'evaluate',
    ts: isoNow(),
    date,
    summary,
    threshold_breach_only: policy.threshold_breach_only,
    surfaced,
    breach_reasons: breach.reasons,
    attention_queue: {
      decision: surfaced ? 'admitted' : 'no_emit',
      queue_path: policy.attention_queue_path
    }
  };

  if (surfaced) {
    const event = {
      event_id: `dopamine_${Date.now()}`,
      source: 'dopamine',
      source_type: 'dopamine_threshold_breach',
      severity: 'warn',
      summary: breach.reasons.length ? `dopamine breach: ${breach.reasons.join(',')}` : 'dopamine breach',
      ts: isoNow(),
      payload: {
        date,
        breach_reasons: breach.reasons,
        summary
      }
    };
    appendJsonl(policy.attention_queue_path, event);
    out.attention_queue = {
      ...out.attention_queue,
      decision: 'admitted',
      event_id: event.event_id,
      queued: true
    };
  }

  out.receipt_hash = hashObject(out);
  return out;
}

async function main() {
  const args = process.argv.slice(2);
  const command = String(args[0] || 'status').trim().toLowerCase();
  const date = parseDateArg(args);
  const summaryRaw = parseSummaryArg(args);

  let payload;
  if (command === 'closeout') {
    payload = closeoutSnapshot(date);
  } else if (command === 'status') {
    payload = statusSnapshot(date);
  } else if (command === 'evaluate') {
    payload = await evaluateSnapshot(date, summaryRaw);
  } else {
    payload = {
      ok: false,
      type: 'dopamine_snapshot_error',
      ts: isoNow(),
      error: 'unknown_command',
      command
    };
    payload.receipt_hash = hashObject(payload);
    process.stdout.write(`${JSON.stringify(payload)}\n`);
    process.exit(2);
    return;
  }

  process.stdout.write(`${JSON.stringify(payload)}\n`);
}

if (require.main === module) {
  main().catch((err) => {
    const payload = {
      ok: false,
      type: 'dopamine_snapshot_error',
      ts: isoNow(),
      error: String(err && err.message ? err.message : err || 'unknown_error')
    };
    payload.receipt_hash = hashObject(payload);
    process.stdout.write(`${JSON.stringify(payload)}\n`);
    process.exit(1);
  });
}

module.exports = {
  closeoutSnapshot,
  statusSnapshot
};
