#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-083
 * Adversarial hypothesis challenger lane.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const ROOT = path.resolve(__dirname, '..', '..');
const POLICY_PATH = process.env.ADVERSARIAL_HYPOTHESIS_CHALLENGER_POLICY_PATH
  ? path.resolve(process.env.ADVERSARIAL_HYPOTHESIS_CHALLENGER_POLICY_PATH)
  : path.join(ROOT, 'config', 'adversarial_hypothesis_challenger_policy.json');

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
    min_priority_confidence: 78,
    min_priority_probability: 0.72,
    min_support_events: 6,
    unresolved_probability_floor: 0.78,
    paths: {
      hypotheses_dir: 'state/sensory/cross_signal/hypotheses',
      output_dir: 'state/sensory/analysis/adversarial_challenger',
      latest_path: 'state/sensory/analysis/adversarial_challenger/latest.json',
      receipts_path: 'state/sensory/analysis/adversarial_challenger/receipts.jsonl'
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
    min_priority_confidence: clampNumber(raw.min_priority_confidence, 1, 100, base.min_priority_confidence),
    min_priority_probability: clampNumber(raw.min_priority_probability, 0, 1, base.min_priority_probability),
    min_support_events: clampNumber(raw.min_support_events, 1, 1000, base.min_support_events),
    unresolved_probability_floor: clampNumber(raw.unresolved_probability_floor, 0, 1, base.unresolved_probability_floor),
    paths: {
      hypotheses_dir: resolvePath(paths.hypotheses_dir, base.paths.hypotheses_dir),
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
  const hypotheses = src && Array.isArray(src.hypotheses) ? src.hypotheses : [];
  return {
    file_path: fp,
    hypotheses: hypotheses.filter((row: any) => row && typeof row === 'object')
  };
}

function highPriority(h: Record<string, any>, policy: Record<string, any>) {
  return Number(h && h.confidence || 0) >= Number(policy.min_priority_confidence || 78)
    && Number(h && h.probability || 0) >= Number(policy.min_priority_probability || 0.72);
}

function challengeReason(h: Record<string, any>) {
  const t = normalizeToken(h && h.type || '', 40);
  if (t === 'convergence') return 'alternative_cause_sampling_bias';
  if (t === 'lead_lag') return 'ordering_artifact_latency_bias';
  if (t === 'temporal_delta') return 'seasonality_or_sampling_artifact';
  if (t === 'negative_signal') return 'complaint_burst_without_actionable_demand';
  return 'insufficient_disconfirmation_evidence';
}

function verifyChallenge(h: Record<string, any>, policy: Record<string, any>) {
  const supportEvents = Number(h && h.support_events || 0);
  const probability = Number(h && h.probability || 0);
  if (supportEvents < Number(policy.min_support_events || 6)) {
    return {
      outcome: 'win',
      reason: 'low_support_events',
      confidence: Number((1 - Math.min(1, supportEvents / Math.max(1, Number(policy.min_support_events || 6)))).toFixed(4))
    };
  }
  if (probability < Number(policy.unresolved_probability_floor || 0.78)) {
    return {
      outcome: 'unresolved',
      reason: 'probability_below_resolution_floor',
      confidence: Number((1 - probability).toFixed(4))
    };
  }
  return {
    outcome: 'loss',
    reason: 'hypothesis_withstood_disconfirmation',
    confidence: Number(probability.toFixed(4))
  };
}

function run(dateStr: string, policy: Record<string, any>, strict = false) {
  const source = loadHypotheses(policy, dateStr);
  const rows = source.hypotheses.filter((h) => highPriority(h, policy));
  const challenges = rows.map((h) => {
    const verification = verifyChallenge(h, policy);
    return {
      challenge_id: `adv_${stableHash(`${dateStr}|${h.id}|${verification.outcome}|${verification.reason}`, 20)}`,
      source_hypothesis_id: cleanText(h.id || '', 160) || null,
      topic: cleanText(h.topic || '', 120),
      source_type: cleanText(h.type || '', 80),
      challenger_claim: challengeReason(h),
      verification_outcome: verification.outcome,
      verification_reason: verification.reason,
      verification_confidence: verification.confidence,
      support_events: Number(h.support_events || 0),
      source_confidence: Number(h.confidence || 0),
      source_probability: Number(h.probability || 0)
    };
  });

  const unresolvedWins = challenges.filter((row) => row.verification_outcome === 'win' || row.verification_outcome === 'unresolved');
  const out = {
    ok: unresolvedWins.length === 0,
    type: 'adversarial_hypothesis_challenger',
    ts: nowIso(),
    date: dateStr,
    source_hypotheses_path: source.file_path,
    source_hypothesis_count: source.hypotheses.length,
    challenged_count: challenges.length,
    unresolved_or_winning_challengers: unresolvedWins.length,
    promotion_blocked: unresolvedWins.length > 0,
    challenges,
    unresolved_examples: unresolvedWins.slice(0, 10)
  };

  ensureDir(policy.paths.output_dir);
  writeJsonAtomic(path.join(policy.paths.output_dir, `${dateStr}.json`), out);
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, {
    ts: nowIso(),
    type: 'adversarial_hypothesis_challenger_receipt',
    date: dateStr,
    challenged_count: challenges.length,
    unresolved_or_winning_challengers: unresolvedWins.length,
    promotion_blocked: out.promotion_blocked
  });

  if (strict && out.promotion_blocked) {
    process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
    process.exit(2);
  }
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
}

function status(policy: Record<string, any>, dateStr: string) {
  const fp = path.join(policy.paths.output_dir, `${dateStr}.json`);
  const payload = readJson(fp, {
    ok: true,
    type: 'adversarial_hypothesis_challenger_status',
    date: dateStr,
    promotion_blocked: false
  });
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
}

function usageAndExit(code = 0) {
  console.log('Usage:');
  console.log('  node systems/sensory/adversarial_hypothesis_challenger.js run [YYYY-MM-DD] [--strict=1] [--policy=<path>]');
  console.log('  node systems/sensory/adversarial_hypothesis_challenger.js status [YYYY-MM-DD] [--policy=<path>]');
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
