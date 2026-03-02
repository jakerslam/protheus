#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-097
 * Ground-truth governance & label adjudication lane.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const ROOT = path.resolve(__dirname, '..', '..');
const POLICY_PATH = process.env.GROUND_TRUTH_ADJUDICATION_POLICY_PATH
  ? path.resolve(process.env.GROUND_TRUTH_ADJUDICATION_POLICY_PATH)
  : path.join(ROOT, 'config', 'ground_truth_label_adjudication_policy.json');

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
    min_reviewer_count: 2,
    min_agreement_ratio: 0.67,
    min_avg_confidence: 0.55,
    paths: {
      raw_labels_dir: 'state/sensory/labels/raw',
      adjudicated_dir: 'state/sensory/labels/adjudicated',
      promotion_corpus_dir: 'state/sensory/labels/promotion_corpus',
      quarantine_dir: 'state/sensory/labels/quarantine',
      latest_path: 'state/sensory/labels/adjudicated/latest.json',
      receipts_path: 'state/sensory/labels/adjudicated/receipts.jsonl'
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
    min_reviewer_count: clampNumber(raw.min_reviewer_count, 1, 20, base.min_reviewer_count),
    min_agreement_ratio: clampNumber(raw.min_agreement_ratio, 0, 1, base.min_agreement_ratio),
    min_avg_confidence: clampNumber(raw.min_avg_confidence, 0, 1, base.min_avg_confidence),
    paths: {
      raw_labels_dir: resolvePath(paths.raw_labels_dir, base.paths.raw_labels_dir),
      adjudicated_dir: resolvePath(paths.adjudicated_dir, base.paths.adjudicated_dir),
      promotion_corpus_dir: resolvePath(paths.promotion_corpus_dir, base.paths.promotion_corpus_dir),
      quarantine_dir: resolvePath(paths.quarantine_dir, base.paths.quarantine_dir),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function loadLabels(policy: Record<string, any>, dateStr: string) {
  const fp = path.join(policy.paths.raw_labels_dir, `${dateStr}.json`);
  const src = readJson(fp, null);
  const rows = src && Array.isArray(src.labels) ? src.labels : [];
  return {
    file_path: fp,
    rows: rows.filter((row: any) => row && typeof row === 'object')
  };
}

function adjudicateOne(row: Record<string, any>, policy: Record<string, any>, dateStr: string) {
  const reviews = Array.isArray(row && row.reviewer_labels) ? row.reviewer_labels : [];
  const normalized = reviews
    .map((rev: any) => ({
      reviewer: cleanText(rev && rev.reviewer || '', 80) || null,
      label: normalizeToken(rev && rev.label || '', 80),
      confidence: clampNumber(rev && rev.confidence, 0, 1, 0.5)
    }))
    .filter((rev: any) => Boolean(rev.label));

  const counts = new Map();
  for (const rev of normalized) {
    counts.set(rev.label, Number(counts.get(rev.label) || 0) + 1);
  }

  const sorted = Array.from(counts.entries()).sort((a, b) => Number(b[1]) - Number(a[1]));
  const winnerLabel = sorted.length > 0 ? String(sorted[0][0]) : null;
  const winnerVotes = sorted.length > 0 ? Number(sorted[0][1]) : 0;
  const reviewerCount = normalized.length;
  const agreementRatio = reviewerCount > 0 ? winnerVotes / reviewerCount : 0;
  const avgConfidence = reviewerCount > 0
    ? normalized.reduce((sum: number, rev: any) => sum + Number(rev.confidence || 0), 0) / reviewerCount
    : 0;

  const quarantineReasons = [];
  if (reviewerCount < Number(policy.min_reviewer_count || 2)) quarantineReasons.push('insufficient_reviewer_count');
  if (agreementRatio < Number(policy.min_agreement_ratio || 0.67)) quarantineReasons.push('agreement_ratio_below_threshold');
  if (avgConfidence < Number(policy.min_avg_confidence || 0.55)) quarantineReasons.push('avg_confidence_below_threshold');

  const accepted = quarantineReasons.length === 0;

  return {
    adjudication_id: `lbl_${stableHash(`${dateStr}|${row.label_id}|${winnerLabel}|${agreementRatio}`, 20)}`,
    label_id: cleanText(row && row.label_id || '', 120) || null,
    example_id: cleanText(row && row.example_id || '', 120) || null,
    winner_label: winnerLabel,
    reviewer_count: reviewerCount,
    agreement_ratio: Number(agreementRatio.toFixed(6)),
    avg_confidence: Number(avgConfidence.toFixed(6)),
    accepted,
    quarantine_reasons: quarantineReasons,
    reviewer_labels: normalized
  };
}

function run(dateStr: string, policy: Record<string, any>, strict = false) {
  const source = loadLabels(policy, dateStr);
  const adjudicated = source.rows.map((row) => adjudicateOne(row, policy, dateStr));
  const accepted = adjudicated.filter((row) => row.accepted === true);
  const quarantined = adjudicated.filter((row) => row.accepted !== true);

  const out = {
    ok: true,
    type: 'ground_truth_label_adjudication_lane',
    ts: nowIso(),
    date: dateStr,
    source_labels_path: source.file_path,
    total_labels: adjudicated.length,
    accepted_labels: accepted.length,
    quarantined_labels: quarantined.length,
    adjudicated
  };

  ensureDir(policy.paths.adjudicated_dir);
  writeJsonAtomic(path.join(policy.paths.adjudicated_dir, `${dateStr}.json`), out);
  writeJsonAtomic(policy.paths.latest_path, out);

  ensureDir(policy.paths.promotion_corpus_dir);
  writeJsonAtomic(path.join(policy.paths.promotion_corpus_dir, `${dateStr}.json`), {
    type: 'promotion_label_corpus',
    ts: nowIso(),
    date: dateStr,
    labels: accepted
  });

  ensureDir(policy.paths.quarantine_dir);
  writeJsonAtomic(path.join(policy.paths.quarantine_dir, `${dateStr}.json`), {
    type: 'quarantined_labels',
    ts: nowIso(),
    date: dateStr,
    labels: quarantined
  });

  appendJsonl(policy.paths.receipts_path, {
    ts: nowIso(),
    type: 'label_adjudication_receipt',
    date: dateStr,
    total_labels: adjudicated.length,
    accepted_labels: accepted.length,
    quarantined_labels: quarantined.length
  });

  if (strict && quarantined.length > 0) {
    process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
    process.exit(2);
  }
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
}

function status(policy: Record<string, any>, dateStr: string) {
  const fp = path.join(policy.paths.adjudicated_dir, `${dateStr}.json`);
  const payload = readJson(fp, {
    ok: true,
    type: 'ground_truth_label_adjudication_lane_status',
    date: dateStr,
    total_labels: 0
  });
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
}

function usageAndExit(code = 0) {
  console.log('Usage:');
  console.log('  node systems/sensory/ground_truth_label_adjudication_lane.js run [YYYY-MM-DD] [--strict=1] [--policy=<path>]');
  console.log('  node systems/sensory/ground_truth_label_adjudication_lane.js status [YYYY-MM-DD] [--policy=<path>]');
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
