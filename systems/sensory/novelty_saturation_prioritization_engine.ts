#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-095
 * Novelty/saturation prioritization engine.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const ROOT = path.resolve(__dirname, '..', '..');
const POLICY_PATH = process.env.NOVELTY_SATURATION_POLICY_PATH
  ? path.resolve(process.env.NOVELTY_SATURATION_POLICY_PATH)
  : path.join(ROOT, 'config', 'novelty_saturation_prioritization_policy.json');

function nowIso() {
  return new Date().toISOString();
}

function todayStr() {
  return new Date().toISOString().slice(0, 10);
}

function cleanText(v: unknown, maxLen = 260) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function normalizeToken(v: unknown, maxLen = 120) {
  return cleanText(v, maxLen)
    .toLowerCase()
    .replace(/[^a-z0-9_.:/-]+/g, '_')
    .replace(/_+/g, '_')
    .replace(/^_+|_+$/g, '');
}

function parseArgs(argv: string[]) {
  const out: Record<string, any> = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const tok = String(argv[i] || '');
    if (!tok.startsWith('--')) {
      out._.push(tok);
      continue;
    }
    const eq = tok.indexOf('=');
    if (eq >= 0) {
      out[tok.slice(2, eq)] = tok.slice(eq + 1);
      continue;
    }
    const key = tok.slice(2);
    const next = argv[i + 1];
    if (next != null && !String(next).startsWith('--')) {
      out[key] = String(next);
      i += 1;
      continue;
    }
    out[key] = true;
  }
  return out;
}

function toBool(v: unknown, fallback = false) {
  if (v == null) return fallback;
  const raw = String(v).trim().toLowerCase();
  if (['1', 'true', 'yes', 'on'].includes(raw)) return true;
  if (['0', 'false', 'no', 'off'].includes(raw)) return false;
  return fallback;
}

function clampNumber(v: unknown, lo: number, hi: number, fallback: number) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  if (n < lo) return lo;
  if (n > hi) return hi;
  return n;
}

function ensureDir(dirPath: string) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function readJson(filePath: string, fallback: any = null) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    const parsed = JSON.parse(fs.readFileSync(filePath, 'utf8'));
    return parsed == null ? fallback : parsed;
  } catch {
    return fallback;
  }
}

function writeJsonAtomic(filePath: string, value: Record<string, any>) {
  ensureDir(path.dirname(filePath));
  const tmp = `${filePath}.tmp-${Date.now()}-${process.pid}`;
  fs.writeFileSync(tmp, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
  fs.renameSync(tmp, filePath);
}

function appendJsonl(filePath: string, row: Record<string, any>) {
  ensureDir(path.dirname(filePath));
  fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`, 'utf8');
}

function resolvePath(raw: unknown, fallbackRel: string) {
  const txt = cleanText(raw, 520);
  if (!txt) return path.join(ROOT, fallbackRel);
  return path.isAbsolute(txt) ? txt : path.join(ROOT, txt);
}

function stableHash(v: unknown, len = 18) {
  return crypto.createHash('sha256').update(String(v == null ? '' : v), 'utf8').digest('hex').slice(0, len);
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    history_days: 14,
    novelty_weight: 0.7,
    saturation_weight: 0.5,
    anomaly_bonus_weight: 0.2,
    min_priority_score: -1,
    paths: {
      hypotheses_dir: 'state/sensory/cross_signal/hypotheses',
      state_path: 'state/sensory/analysis/novelty_saturation/state.json',
      output_dir: 'state/sensory/analysis/novelty_saturation',
      latest_path: 'state/sensory/analysis/novelty_saturation/latest.json',
      receipts_path: 'state/sensory/analysis/novelty_saturation/receipts.jsonl'
    }
  };
}

function loadPolicy(policyPath = POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 32) || base.version,
    enabled: raw.enabled !== false,
    history_days: clampNumber(raw.history_days, 1, 120, base.history_days),
    novelty_weight: clampNumber(raw.novelty_weight, 0, 2, base.novelty_weight),
    saturation_weight: clampNumber(raw.saturation_weight, 0, 2, base.saturation_weight),
    anomaly_bonus_weight: clampNumber(raw.anomaly_bonus_weight, 0, 2, base.anomaly_bonus_weight),
    min_priority_score: clampNumber(raw.min_priority_score, -1, 1, base.min_priority_score),
    paths: {
      hypotheses_dir: resolvePath(paths.hypotheses_dir, base.paths.hypotheses_dir),
      state_path: resolvePath(paths.state_path, base.paths.state_path),
      output_dir: resolvePath(paths.output_dir, base.paths.output_dir),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function loadHypotheses(policy: Record<string, any>, dateStr: string) {
  const fp = path.join(policy.paths.hypotheses_dir, `${dateStr}.json`);
  const src = readJson(fp, null);
  const rows = src && Array.isArray(src.hypotheses) ? src.hypotheses : [];
  return {
    file_path: fp,
    rows: rows.filter((row: any) => row && typeof row === 'object')
  };
}

function loadState(policy: Record<string, any>) {
  return readJson(policy.paths.state_path, {
    schema_id: 'novelty_saturation_state',
    version: '1.0',
    history: {}
  });
}

function average(arr: number[]) {
  if (!Array.isArray(arr) || arr.length === 0) return 0;
  return arr.reduce((sum, row) => sum + row, 0) / arr.length;
}

function run(dateStr: string, policy: Record<string, any>, strict = false) {
  const source = loadHypotheses(policy, dateStr);
  const state = loadState(policy);
  const history = state && state.history && typeof state.history === 'object' ? state.history : {};

  const counts: Record<string, number> = {};
  for (const row of source.rows) {
    const topic = normalizeToken(row && row.topic || '', 120);
    if (!topic) continue;
    counts[topic] = Number(counts[topic] || 0) + 1;
  }

  const rows = [];
  const nextHistory: Record<string, any> = { ...history };

  for (const [topic, todayCountRaw] of Object.entries(counts)) {
    const todayCount = Number(todayCountRaw || 0);
    const series = Array.isArray(history[topic]) ? history[topic].map((v: any) => Number(v || 0)).filter((v: number) => Number.isFinite(v)) : [];
    const baseline = average(series);
    const saturation = clampNumber(baseline / Math.max(1, baseline + todayCount), 0, 1, 0);
    const novelty = clampNumber(1 - Math.min(1, baseline / Math.max(1, todayCount)), 0, 1, 1);
    const anomaly = clampNumber((todayCount - baseline) / Math.max(1, todayCount + baseline), -1, 1, 0);

    const priorityScore = clampNumber(
      (novelty * Number(policy.novelty_weight || 0.7))
      - (saturation * Number(policy.saturation_weight || 0.5))
      + (anomaly * Number(policy.anomaly_bonus_weight || 0.2)),
      -1,
      1,
      0
    );

    rows.push({
      topic,
      today_count: todayCount,
      baseline_count: Number(baseline.toFixed(6)),
      novelty_score: Number(novelty.toFixed(6)),
      saturation_score: Number(saturation.toFixed(6)),
      anomaly_score: Number(anomaly.toFixed(6)),
      priority_score: Number(priorityScore.toFixed(6)),
      action: priorityScore >= Number(policy.min_priority_score || -1)
        ? 'prioritize'
        : 'defer'
    });

    const updated = [...series, todayCount].slice(-Number(policy.history_days || 14));
    nextHistory[topic] = updated;
  }

  rows.sort((a, b) => Number(b.priority_score || 0) - Number(a.priority_score || 0));

  const out = {
    ok: true,
    type: 'novelty_saturation_prioritization_engine',
    ts: nowIso(),
    date: dateStr,
    source_hypotheses_path: source.file_path,
    topic_count: rows.length,
    prioritized_topics: rows.filter((row) => row.action === 'prioritize').length,
    scores: rows
  };

  ensureDir(path.dirname(policy.paths.state_path));
  writeJsonAtomic(policy.paths.state_path, {
    schema_id: 'novelty_saturation_state',
    version: String(policy.version || '1.0'),
    updated_at: nowIso(),
    history: nextHistory
  });

  ensureDir(policy.paths.output_dir);
  writeJsonAtomic(path.join(policy.paths.output_dir, `${dateStr}.json`), out);
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, {
    ts: nowIso(),
    type: 'novelty_saturation_receipt',
    date: dateStr,
    topic_count: out.topic_count,
    prioritized_topics: out.prioritized_topics,
    top_topic: rows[0] ? rows[0].topic : null,
    top_priority_score: rows[0] ? rows[0].priority_score : 0,
    receipt_id: `ns_${stableHash(`${dateStr}|${rows[0] ? rows[0].topic : 'none'}|${rows.length}`, 20)}`
  });

  if (strict && rows.length === 0) {
    process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
    process.exit(2);
  }
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
}

function status(policy: Record<string, any>, dateStr: string) {
  const fp = path.join(policy.paths.output_dir, `${dateStr}.json`);
  const payload = readJson(fp, {
    ok: true,
    type: 'novelty_saturation_prioritization_engine_status',
    date: dateStr,
    topic_count: 0
  });
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
}

function usageAndExit(code = 0) {
  console.log('Usage:');
  console.log('  node systems/sensory/novelty_saturation_prioritization_engine.js run [YYYY-MM-DD] [--strict=1] [--policy=<path>]');
  console.log('  node systems/sensory/novelty_saturation_prioritization_engine.js status [YYYY-MM-DD] [--policy=<path>]');
  process.exit(code);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'status', 40).toLowerCase() || 'status';
  const dateStr = /^\d{4}-\d{2}-\d{2}$/.test(String(args._[1] || '')) ? String(args._[1]) : todayStr();
  const strict = toBool(args.strict, false);
  const policy = loadPolicy(args.policy ? String(args.policy) : undefined);
  if (policy.enabled !== true) {
    process.stdout.write(`${JSON.stringify({ ok: false, error: 'policy_disabled' }, null, 2)}\n`);
    process.exit(2);
  }
  if (cmd === 'run') return run(dateStr, policy, strict);
  if (cmd === 'status') return status(policy, dateStr);
  return usageAndExit(2);
}

module.exports = {
  run
};

if (require.main === module) {
  main();
}
