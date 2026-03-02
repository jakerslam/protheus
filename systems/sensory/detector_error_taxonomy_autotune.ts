#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-093
 * Detector error taxonomy + guarded auto-retuning.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const ROOT = path.resolve(__dirname, '..', '..');
const POLICY_PATH = process.env.DETECTOR_ERROR_AUTOTUNE_POLICY_PATH
  ? path.resolve(process.env.DETECTOR_ERROR_AUTOTUNE_POLICY_PATH)
  : path.join(ROOT, 'config', 'detector_error_taxonomy_autotune_policy.json');

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
    probability_key: 'challenger_probability',
    decision_threshold_default: 0.5,
    fp_target_rate: 0.12,
    fn_target_rate: 0.12,
    max_threshold_step: 0.04,
    regression_f1_tolerance: 0.005,
    rollback_on_regression: true,
    paths: {
      eval_pack_dir: 'state/sensory/eval/champion_challenger',
      state_path: 'state/sensory/analysis/detector_autotune/state.json',
      output_dir: 'state/sensory/analysis/detector_autotune',
      latest_path: 'state/sensory/analysis/detector_autotune/latest.json',
      receipts_path: 'state/sensory/analysis/detector_autotune/receipts.jsonl'
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
    probability_key: cleanText(raw.probability_key || base.probability_key, 80) || base.probability_key,
    decision_threshold_default: clampNumber(raw.decision_threshold_default, 0, 1, base.decision_threshold_default),
    fp_target_rate: clampNumber(raw.fp_target_rate, 0, 1, base.fp_target_rate),
    fn_target_rate: clampNumber(raw.fn_target_rate, 0, 1, base.fn_target_rate),
    max_threshold_step: clampNumber(raw.max_threshold_step, 0, 1, base.max_threshold_step),
    regression_f1_tolerance: clampNumber(raw.regression_f1_tolerance, 0, 1, base.regression_f1_tolerance),
    rollback_on_regression: raw.rollback_on_regression !== false,
    paths: {
      eval_pack_dir: resolvePath(paths.eval_pack_dir, base.paths.eval_pack_dir),
      state_path: resolvePath(paths.state_path, base.paths.state_path),
      output_dir: resolvePath(paths.output_dir, base.paths.output_dir),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function loadPack(policy: Record<string, any>, dateStr: string) {
  const fp = path.join(policy.paths.eval_pack_dir, `${dateStr}.json`);
  const src = readJson(fp, null);
  const items = src && Array.isArray(src.items) ? src.items : [];
  return {
    file_path: fp,
    detector_id: cleanText(src && src.challenger_id || src && src.detector_id || 'detector', 120),
    items: items.filter((row: any) => row && typeof row === 'object')
  };
}

function loadState(policy: Record<string, any>) {
  return readJson(policy.paths.state_path, {
    schema_id: 'detector_autotune_state',
    version: '1.0',
    threshold: policy.decision_threshold_default,
    last_metrics: null,
    last_tuned_at: null
  });
}

function score(items: Record<string, any>[], threshold: number, key: string) {
  let tp = 0;
  let fp = 0;
  let tn = 0;
  let fn = 0;
  for (const row of items || []) {
    const truth = Number(row.truth || 0) >= 0.5 ? 1 : 0;
    const prob = clampNumber(row[key], 0, 1, 0);
    const pred = prob >= threshold ? 1 : 0;
    if (pred === 1 && truth === 1) tp += 1;
    else if (pred === 1 && truth === 0) fp += 1;
    else if (pred === 0 && truth === 0) tn += 1;
    else if (pred === 0 && truth === 1) fn += 1;
  }
  const precision = (tp + fp) > 0 ? tp / (tp + fp) : 0;
  const recall = (tp + fn) > 0 ? tp / (tp + fn) : 0;
  const f1 = (precision + recall) > 0 ? (2 * precision * recall) / (precision + recall) : 0;
  const fpRate = (fp + tn) > 0 ? fp / (fp + tn) : 0;
  const fnRate = (fn + tp) > 0 ? fn / (fn + tp) : 0;
  return {
    sample_count: items.length,
    confusion: { tp, fp, tn, fn },
    precision: Number(precision.toFixed(6)),
    recall: Number(recall.toFixed(6)),
    f1: Number(f1.toFixed(6)),
    fp_rate: Number(fpRate.toFixed(6)),
    fn_rate: Number(fnRate.toFixed(6))
  };
}

function taxonomy(items: Record<string, any>[], threshold: number, key: string) {
  const buckets: Record<string, number> = {
    false_positive_high_confidence: 0,
    false_positive_borderline: 0,
    false_negative_high_miss: 0,
    false_negative_borderline: 0
  };

  for (const row of items || []) {
    const truth = Number(row.truth || 0) >= 0.5 ? 1 : 0;
    const prob = clampNumber(row[key], 0, 1, 0);
    const pred = prob >= threshold ? 1 : 0;
    if (pred === 1 && truth === 0) {
      if (prob >= 0.8) buckets.false_positive_high_confidence += 1;
      else buckets.false_positive_borderline += 1;
    }
    if (pred === 0 && truth === 1) {
      if (prob <= 0.2) buckets.false_negative_high_miss += 1;
      else buckets.false_negative_borderline += 1;
    }
  }
  return buckets;
}

function proposeThreshold(currentThreshold: number, metrics: Record<string, any>, policy: Record<string, any>) {
  let step = 0;
  if (Number(metrics.fp_rate || 0) > Number(policy.fp_target_rate || 0.12)) {
    step += Number(policy.max_threshold_step || 0.04) * 0.5;
  }
  if (Number(metrics.fn_rate || 0) > Number(policy.fn_target_rate || 0.12)) {
    step -= Number(policy.max_threshold_step || 0.04) * 0.5;
  }
  step = clampNumber(step, -Number(policy.max_threshold_step || 0.04), Number(policy.max_threshold_step || 0.04), 0);
  return clampNumber(currentThreshold + step, 0.01, 0.99, currentThreshold);
}

function run(dateStr: string, policy: Record<string, any>, strict = false) {
  const pack = loadPack(policy, dateStr);
  const state = loadState(policy);
  const currentThreshold = clampNumber(state.threshold, 0.01, 0.99, Number(policy.decision_threshold_default || 0.5));

  const before = score(pack.items, currentThreshold, String(policy.probability_key || 'challenger_probability'));
  const bucketCounts = taxonomy(pack.items, currentThreshold, String(policy.probability_key || 'challenger_probability'));

  const proposedThreshold = proposeThreshold(currentThreshold, before, policy);
  const after = score(pack.items, proposedThreshold, String(policy.probability_key || 'challenger_probability'));

  const f1Delta = Number((Number(after.f1 || 0) - Number(before.f1 || 0)).toFixed(6));
  const regression = f1Delta < -Number(policy.regression_f1_tolerance || 0.005);
  const rollbackTriggered = Boolean(policy.rollback_on_regression) && regression;
  const appliedThreshold = rollbackTriggered ? currentThreshold : proposedThreshold;
  const appliedMetrics = rollbackTriggered ? before : after;

  const out = {
    ok: true,
    type: 'detector_error_taxonomy_autotune',
    ts: nowIso(),
    date: dateStr,
    detector_id: pack.detector_id,
    eval_pack_path: pack.file_path,
    threshold: {
      before: Number(currentThreshold.toFixed(6)),
      proposed: Number(proposedThreshold.toFixed(6)),
      applied: Number(appliedThreshold.toFixed(6)),
      rollback_triggered: rollbackTriggered
    },
    metrics: {
      before,
      proposed_after: after,
      applied: appliedMetrics,
      f1_delta: f1Delta
    },
    taxonomy: bucketCounts,
    policy_targets: {
      fp_target_rate: Number(policy.fp_target_rate || 0.12),
      fn_target_rate: Number(policy.fn_target_rate || 0.12)
    },
    tuning_receipt_id: `detune_${stableHash(`${dateStr}|${pack.detector_id}|${currentThreshold}|${appliedThreshold}|${f1Delta}`, 20)}`
  };

  ensureDir(path.dirname(policy.paths.state_path));
  writeJsonAtomic(policy.paths.state_path, {
    schema_id: 'detector_autotune_state',
    version: String(policy.version || '1.0'),
    threshold: Number(appliedThreshold.toFixed(6)),
    last_metrics: appliedMetrics,
    last_tuned_at: nowIso(),
    rollback_triggered: rollbackTriggered,
    tuning_receipt_id: out.tuning_receipt_id
  });

  ensureDir(policy.paths.output_dir);
  writeJsonAtomic(path.join(policy.paths.output_dir, `${dateStr}.json`), out);
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, {
    ts: nowIso(),
    type: 'detector_error_autotune_receipt',
    date: dateStr,
    detector_id: pack.detector_id,
    threshold: out.threshold,
    f1_delta: out.metrics.f1_delta,
    rollback_triggered: rollbackTriggered,
    tuning_receipt_id: out.tuning_receipt_id
  });

  if (strict && rollbackTriggered) {
    process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
    process.exit(2);
  }
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
}

function status(policy: Record<string, any>, dateStr: string) {
  const fp = path.join(policy.paths.output_dir, `${dateStr}.json`);
  const payload = readJson(fp, {
    ok: true,
    type: 'detector_error_taxonomy_autotune_status',
    date: dateStr,
    threshold: { applied: policy.decision_threshold_default }
  });
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
}

function usageAndExit(code = 0) {
  console.log('Usage:');
  console.log('  node systems/sensory/detector_error_taxonomy_autotune.js run [YYYY-MM-DD] [--strict=1] [--policy=<path>]');
  console.log('  node systems/sensory/detector_error_taxonomy_autotune.js status [YYYY-MM-DD] [--policy=<path>]');
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
