#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-088
 * Abstain/uncertainty output contract.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const ROOT = path.resolve(__dirname, '..', '..');
const POLICY_PATH = process.env.ABSTAIN_UNCERTAINTY_POLICY_PATH
  ? path.resolve(process.env.ABSTAIN_UNCERTAINTY_POLICY_PATH)
  : path.join(ROOT, 'config', 'abstain_uncertainty_contract_policy.json');

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
    abstain_if_confidence_below: 72,
    abstain_if_probability_below: 0.67,
    abstain_if_support_events_below: 4,
    reason_codes: {
      low_confidence: 'insufficient_confidence',
      low_probability: 'insufficient_probability',
      low_support: 'insufficient_support_events'
    },
    paths: {
      hypotheses_dir: 'state/sensory/cross_signal/hypotheses',
      resolution_dir: 'state/sensory/analysis/abstain_resolution',
      output_dir: 'state/sensory/analysis/abstain_uncertainty',
      latest_path: 'state/sensory/analysis/abstain_uncertainty/latest.json',
      receipts_path: 'state/sensory/analysis/abstain_uncertainty/receipts.jsonl'
    }
  };
}

function loadPolicy(policyPath = POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  const reasonCodes = raw.reason_codes && typeof raw.reason_codes === 'object' ? raw.reason_codes : {};
  return {
    version: cleanText(raw.version || base.version, 32) || base.version,
    enabled: raw.enabled !== false,
    abstain_if_confidence_below: clampNumber(raw.abstain_if_confidence_below, 0, 100, base.abstain_if_confidence_below),
    abstain_if_probability_below: clampNumber(raw.abstain_if_probability_below, 0, 1, base.abstain_if_probability_below),
    abstain_if_support_events_below: clampNumber(raw.abstain_if_support_events_below, 0, 1000, base.abstain_if_support_events_below),
    reason_codes: {
      low_confidence: normalizeToken(reasonCodes.low_confidence || base.reason_codes.low_confidence, 80),
      low_probability: normalizeToken(reasonCodes.low_probability || base.reason_codes.low_probability, 80),
      low_support: normalizeToken(reasonCodes.low_support || base.reason_codes.low_support, 80)
    },
    paths: {
      hypotheses_dir: resolvePath(paths.hypotheses_dir, base.paths.hypotheses_dir),
      resolution_dir: resolvePath(paths.resolution_dir, base.paths.resolution_dir),
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

function loadResolution(policy: Record<string, any>, dateStr: string) {
  const fp = path.join(policy.paths.resolution_dir, `${dateStr}.json`);
  const src = readJson(fp, null);
  const rows = src && Array.isArray(src.resolutions) ? src.resolutions : [];
  const map = new Map();
  for (const row of rows) {
    const abstainId = cleanText(row && row.abstain_id || '', 160);
    if (!abstainId) continue;
    map.set(abstainId, {
      resolved: Boolean(row && row.resolved),
      outcome: cleanText(row && row.outcome || '', 80) || null,
      resolution_ts: cleanText(row && row.resolution_ts || '', 60) || null
    });
  }
  return {
    file_path: fp,
    map
  };
}

function abstainReasons(h: Record<string, any>, policy: Record<string, any>) {
  const reasons = [];
  const confidence = Number(h && h.confidence || 0);
  const probability = Number(h && h.probability || 0);
  const supportEvents = Number(h && h.support_events || 0);
  if (confidence < Number(policy.abstain_if_confidence_below || 72)) reasons.push(policy.reason_codes.low_confidence);
  if (probability < Number(policy.abstain_if_probability_below || 0.67)) reasons.push(policy.reason_codes.low_probability);
  if (supportEvents < Number(policy.abstain_if_support_events_below || 4)) reasons.push(policy.reason_codes.low_support);
  return reasons;
}

function run(dateStr: string, policy: Record<string, any>, strict = false) {
  const source = loadHypotheses(policy, dateStr);
  const resolution = loadResolution(policy, dateStr);

  const abstained = [];
  const routed = [];

  for (const h of source.hypotheses) {
    const reasons = abstainReasons(h, policy);
    if (reasons.length > 0) {
      const abstainId = `abs_${stableHash(`${dateStr}|${h.id}|${reasons.join('|')}`, 20)}`;
      const resolved = resolution.map.get(abstainId) || null;
      abstained.push({
        abstain_id: abstainId,
        source_hypothesis_id: cleanText(h && h.id || '', 160) || null,
        topic: cleanText(h && h.topic || '', 120) || null,
        confidence: Number(h && h.confidence || 0),
        probability: Number(h && h.probability || 0),
        support_events: Number(h && h.support_events || 0),
        reason_codes: reasons,
        routing: 'abstain_review_queue',
        resolved: resolved ? resolved.resolved : false,
        resolution_outcome: resolved ? resolved.outcome : null,
        resolution_ts: resolved ? resolved.resolution_ts : null
      });
    } else {
      routed.push({
        source_hypothesis_id: cleanText(h && h.id || '', 160) || null,
        topic: cleanText(h && h.topic || '', 120) || null,
        routing: 'normal_promotion_path'
      });
    }
  }

  const resolvedCount = abstained.filter((row) => row.resolved === true).length;
  const resolutionRate = abstained.length > 0 ? resolvedCount / abstained.length : 1;

  const out = {
    ok: true,
    type: 'abstain_uncertainty_contract',
    ts: nowIso(),
    date: dateStr,
    source_hypotheses_path: source.file_path,
    resolution_path: resolution.file_path,
    source_hypothesis_count: source.hypotheses.length,
    abstain_count: abstained.length,
    routed_count: routed.length,
    abstain_resolution: {
      resolved_count: resolvedCount,
      unresolved_count: abstained.length - resolvedCount,
      resolution_rate: Number(resolutionRate.toFixed(6))
    },
    abstained,
    routed
  };

  ensureDir(policy.paths.output_dir);
  writeJsonAtomic(path.join(policy.paths.output_dir, `${dateStr}.json`), out);
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, {
    ts: nowIso(),
    type: 'abstain_uncertainty_contract_receipt',
    date: dateStr,
    source_hypothesis_count: source.hypotheses.length,
    abstain_count: abstained.length,
    routed_count: routed.length,
    abstain_resolution: out.abstain_resolution
  });

  if (strict && abstained.length === 0) {
    process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
    process.exit(2);
  }
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
}

function status(policy: Record<string, any>, dateStr: string) {
  const fp = path.join(policy.paths.output_dir, `${dateStr}.json`);
  const payload = readJson(fp, {
    ok: true,
    type: 'abstain_uncertainty_contract_status',
    date: dateStr,
    abstain_count: 0
  });
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
}

function usageAndExit(code = 0) {
  console.log('Usage:');
  console.log('  node systems/sensory/abstain_uncertainty_contract.js run [YYYY-MM-DD] [--strict=1] [--policy=<path>]');
  console.log('  node systems/sensory/abstain_uncertainty_contract.js status [YYYY-MM-DD] [--policy=<path>]');
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
