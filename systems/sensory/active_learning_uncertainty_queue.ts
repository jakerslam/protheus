#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-101
 * Active-learning uncertainty queue.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const ROOT = path.resolve(__dirname, '..', '..');
const POLICY_PATH = process.env.ACTIVE_LEARNING_QUEUE_POLICY_PATH
  ? path.resolve(process.env.ACTIVE_LEARNING_QUEUE_POLICY_PATH)
  : path.join(ROOT, 'config', 'active_learning_uncertainty_queue_policy.json');

function nowIso() {
  return new Date().toISOString();
}

function todayStr() {
  return new Date().toISOString().slice(0, 10);
}

function cleanText(v: unknown, maxLen = 260) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
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
    max_queue_items: 200,
    min_priority: 0.1,
    paths: {
      abstain_dir: 'state/sensory/analysis/abstain_uncertainty',
      disagreement_dir: 'state/sensory/analysis/ensemble_disagreement',
      label_promotion_dir: 'state/sensory/labels/promotion_corpus',
      queue_path: 'state/sensory/analysis/active_learning/queue.jsonl',
      output_dir: 'state/sensory/analysis/active_learning',
      latest_path: 'state/sensory/analysis/active_learning/latest.json',
      receipts_path: 'state/sensory/analysis/active_learning/receipts.jsonl'
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
    max_queue_items: Number.isFinite(Number(raw.max_queue_items)) ? Number(raw.max_queue_items) : base.max_queue_items,
    min_priority: Number.isFinite(Number(raw.min_priority)) ? Number(raw.min_priority) : base.min_priority,
    paths: {
      abstain_dir: resolvePath(paths.abstain_dir, base.paths.abstain_dir),
      disagreement_dir: resolvePath(paths.disagreement_dir, base.paths.disagreement_dir),
      label_promotion_dir: resolvePath(paths.label_promotion_dir, base.paths.label_promotion_dir),
      queue_path: resolvePath(paths.queue_path, base.paths.queue_path),
      output_dir: resolvePath(paths.output_dir, base.paths.output_dir),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function run(dateStr: string, policy: Record<string, any>, strict = false) {
  const abstain = readJson(path.join(policy.paths.abstain_dir, `${dateStr}.json`), { abstained: [] });
  const disagreement = readJson(path.join(policy.paths.disagreement_dir, `${dateStr}.json`), { adjudication_queue: [] });
  const labels = readJson(path.join(policy.paths.label_promotion_dir, `${dateStr}.json`), { labels: [] });

  const queueItems = [];
  const seen = new Set();

  for (const row of Array.isArray(abstain.abstained) ? abstain.abstained : []) {
    const key = cleanText(row && row.abstain_id || row && row.source_hypothesis_id || '', 120);
    if (!key || seen.has(key)) continue;
    seen.add(key);
    const priority = Math.min(1, 0.4 + (Array.isArray(row.reason_codes) ? row.reason_codes.length * 0.15 : 0));
    if (priority < Number(policy.min_priority || 0.1)) continue;
    queueItems.push({
      queue_id: `alq_${stableHash(`${dateStr}|abstain|${key}`, 20)}`,
      source_type: 'abstain_uncertainty',
      source_ref: key,
      topic: cleanText(row && row.topic || '', 120) || null,
      priority: Number(priority.toFixed(6)),
      reason: 'abstain_resolution_needed',
      route: 'label_review'
    });
  }

  for (const row of Array.isArray(disagreement.adjudication_queue) ? disagreement.adjudication_queue : []) {
    const key = cleanText(row && row.adjudication_id || row && row.item_id || '', 120);
    if (!key || seen.has(key)) continue;
    seen.add(key);
    const priority = Math.min(1, 0.45 + Number(row.disagreement_stddev || 0));
    if (priority < Number(policy.min_priority || 0.1)) continue;
    queueItems.push({
      queue_id: `alq_${stableHash(`${dateStr}|ensemble|${key}`, 20)}`,
      source_type: 'ensemble_disagreement',
      source_ref: key,
      topic: cleanText(row && row.topic || row && row.item_id || '', 120) || null,
      priority: Number(priority.toFixed(6)),
      reason: 'ensemble_divergence_adjudication',
      route: 'label_review'
    });
  }

  queueItems.sort((a, b) => Number(b.priority || 0) - Number(a.priority || 0));
  const queued = queueItems.slice(0, Number(policy.max_queue_items || 200));

  for (const row of queued) {
    appendJsonl(policy.paths.queue_path, {
      ts: nowIso(),
      date: dateStr,
      ...row
    });
  }

  const out = {
    ok: true,
    type: 'active_learning_uncertainty_queue',
    ts: nowIso(),
    date: dateStr,
    queued_count: queued.length,
    accepted_label_feedback_count: Array.isArray(labels.labels) ? labels.labels.length : 0,
    queued,
    expected_model_uplift_hint: Number((Math.min(0.2, queued.length * 0.01 + ((Array.isArray(labels.labels) ? labels.labels.length : 0) * 0.001))).toFixed(6))
  };

  ensureDir(policy.paths.output_dir);
  writeJsonAtomic(path.join(policy.paths.output_dir, `${dateStr}.json`), out);
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, {
    ts: nowIso(),
    type: 'active_learning_queue_receipt',
    date: dateStr,
    queued_count: queued.length,
    accepted_label_feedback_count: out.accepted_label_feedback_count,
    expected_model_uplift_hint: out.expected_model_uplift_hint
  });

  if (strict && queued.length === 0) {
    process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
    process.exit(2);
  }
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
}

function status(policy: Record<string, any>, dateStr: string) {
  const fp = path.join(policy.paths.output_dir, `${dateStr}.json`);
  const payload = readJson(fp, {
    ok: true,
    type: 'active_learning_uncertainty_queue_status',
    date: dateStr,
    queued_count: 0
  });
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
}

function usageAndExit(code = 0) {
  console.log('Usage:');
  console.log('  node systems/sensory/active_learning_uncertainty_queue.js run [YYYY-MM-DD] [--strict=1] [--policy=<path>]');
  console.log('  node systems/sensory/active_learning_uncertainty_queue.js status [YYYY-MM-DD] [--policy=<path>]');
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
