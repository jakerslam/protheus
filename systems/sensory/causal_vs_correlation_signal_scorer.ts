#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-090
 * Causal-vs-correlational signal scorer.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const ROOT = path.resolve(__dirname, '..', '..');
const POLICY_PATH = process.env.CAUSAL_SIGNAL_SCORER_POLICY_PATH
  ? path.resolve(process.env.CAUSAL_SIGNAL_SCORER_POLICY_PATH)
  : path.join(ROOT, 'config', 'causal_vs_correlation_signal_scorer_policy.json');

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
    min_causal_score: 0.58,
    max_correlation_penalty: 0.22,
    weights: {
      chain_confidence: 0.42,
      counterfactual_uplift: 0.28,
      source_reliability: 0.2,
      structure_bonus: 0.1
    },
    paths: {
      chain_mapper_dir: 'state/sensory/analysis/objective_chain_mapper',
      counterfactual_dir: 'state/sensory/analysis/counterfactual_replay',
      source_reliability_latest: 'state/sensory/analysis/source_reliability/latest.json',
      output_dir: 'state/sensory/analysis/causal_signal_scorer',
      latest_path: 'state/sensory/analysis/causal_signal_scorer/latest.json',
      receipts_path: 'state/sensory/analysis/causal_signal_scorer/receipts.jsonl'
    }
  };
}

function loadPolicy(policyPath = POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  const weights = raw.weights && typeof raw.weights === 'object' ? raw.weights : {};
  return {
    version: cleanText(raw.version || base.version, 32) || base.version,
    enabled: raw.enabled !== false,
    min_causal_score: clampNumber(raw.min_causal_score, 0, 1, base.min_causal_score),
    max_correlation_penalty: clampNumber(raw.max_correlation_penalty, 0, 1, base.max_correlation_penalty),
    weights: {
      chain_confidence: clampNumber(weights.chain_confidence, 0, 1, base.weights.chain_confidence),
      counterfactual_uplift: clampNumber(weights.counterfactual_uplift, 0, 1, base.weights.counterfactual_uplift),
      source_reliability: clampNumber(weights.source_reliability, 0, 1, base.weights.source_reliability),
      structure_bonus: clampNumber(weights.structure_bonus, 0, 1, base.weights.structure_bonus)
    },
    paths: {
      chain_mapper_dir: resolvePath(paths.chain_mapper_dir, base.paths.chain_mapper_dir),
      counterfactual_dir: resolvePath(paths.counterfactual_dir, base.paths.counterfactual_dir),
      source_reliability_latest: resolvePath(paths.source_reliability_latest, base.paths.source_reliability_latest),
      output_dir: resolvePath(paths.output_dir, base.paths.output_dir),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function loadChainRows(policy: Record<string, any>, dateStr: string) {
  const fp = path.join(policy.paths.chain_mapper_dir, `${dateStr}.json`);
  const src = readJson(fp, null);
  const rows = src && Array.isArray(src.chains) ? src.chains : [];
  return {
    file_path: fp,
    rows: rows.filter((row: any) => row && typeof row === 'object')
  };
}

function loadCounterfactual(policy: Record<string, any>, dateStr: string) {
  const fp = path.join(policy.paths.counterfactual_dir, `${dateStr}.json`);
  const src = readJson(fp, null);
  const deltas = src && src.deltas && typeof src.deltas === 'object' ? src.deltas : {};
  return {
    file_path: fp,
    precision_uplift: clampNumber(deltas.precision_uplift, -1, 1, 0),
    recall_uplift: clampNumber(deltas.recall_uplift, -1, 1, 0)
  };
}

function loadSourceReliability(policy: Record<string, any>) {
  const src = readJson(policy.paths.source_reliability_latest, null);
  const rows = src && Array.isArray(src.sources) ? src.sources : [];
  const map = new Map();
  for (const row of rows) {
    const sourceId = cleanText(row && row.source_id || '', 120);
    if (!sourceId) continue;
    map.set(sourceId, clampNumber(row && row.score, 0, 1, 0.5));
  }
  return {
    file_path: policy.paths.source_reliability_latest,
    map
  };
}

function run(dateStr: string, policy: Record<string, any>, strict = false) {
  const chainSrc = loadChainRows(policy, dateStr);
  const cf = loadCounterfactual(policy, dateStr);
  const sourceReliability = loadSourceReliability(policy);
  const rows = [];

  const cfUpliftNorm = clampNumber(((cf.precision_uplift + cf.recall_uplift) / 2 + 1) / 2, 0, 1, 0.5);

  for (const chain of chainSrc.rows) {
    const corr = clampNumber(chain && chain.path_confidence, 0, 1, 0);
    const eyeId = cleanText(chain && chain.eye_id || 'unknown_source', 120) || 'unknown_source';
    const sourceScore = clampNumber(sourceReliability.map.get(eyeId), 0, 1, 0.5);
    const hopCount = Array.isArray(chain && chain.hops) ? chain.hops.length : 0;
    const structureBonus = hopCount >= 4 ? 1 : hopCount / 4;

    const causalScore = clampNumber(
      corr * Number(policy.weights.chain_confidence || 0.42)
      + cfUpliftNorm * Number(policy.weights.counterfactual_uplift || 0.28)
      + sourceScore * Number(policy.weights.source_reliability || 0.2)
      + structureBonus * Number(policy.weights.structure_bonus || 0.1),
      0,
      1,
      0
    );

    const correlationOnlyPenalty = causalScore < Number(policy.min_causal_score || 0.58)
      ? clampNumber((Number(policy.min_causal_score || 0.58) - causalScore), 0, Number(policy.max_correlation_penalty || 0.22), 0)
      : 0;

    const finalScore = clampNumber(corr - correlationOnlyPenalty + (causalScore * 0.1), 0, 1, 0);

    rows.push({
      path_id: cleanText(chain && chain.path_id || `path_${stableHash(JSON.stringify(chain), 12)}`, 160),
      eye_id: eyeId,
      objective_id: cleanText(chain && chain.objective_id || '', 120) || null,
      correlation_score: Number(corr.toFixed(6)),
      causal_score: Number(causalScore.toFixed(6)),
      correlation_only_penalty: Number(correlationOnlyPenalty.toFixed(6)),
      final_score: Number(finalScore.toFixed(6)),
      evidence: {
        counterfactual_uplift_norm: Number(cfUpliftNorm.toFixed(6)),
        source_reliability: Number(sourceScore.toFixed(6)),
        structure_bonus: Number(structureBonus.toFixed(6))
      }
    });
  }

  rows.sort((a, b) => Number(b.final_score || 0) - Number(a.final_score || 0));
  const penalized = rows.filter((row) => Number(row.correlation_only_penalty || 0) > 0).length;

  const out = {
    ok: true,
    type: 'causal_vs_correlation_signal_scorer',
    ts: nowIso(),
    date: dateStr,
    source_paths: {
      chain_mapper: chainSrc.file_path,
      counterfactual: cf.file_path,
      source_reliability: sourceReliability.file_path
    },
    chain_count: rows.length,
    penalized_count: penalized,
    min_causal_score: Number(policy.min_causal_score || 0.58),
    rankings: rows
  };

  ensureDir(policy.paths.output_dir);
  writeJsonAtomic(path.join(policy.paths.output_dir, `${dateStr}.json`), out);
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, {
    ts: nowIso(),
    type: 'causal_vs_correlation_scorer_receipt',
    date: dateStr,
    chain_count: rows.length,
    penalized_count: penalized,
    top_path_id: rows[0] ? rows[0].path_id : null,
    top_final_score: rows[0] ? rows[0].final_score : 0
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
    type: 'causal_vs_correlation_signal_scorer_status',
    date: dateStr,
    chain_count: 0
  });
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
}

function usageAndExit(code = 0) {
  console.log('Usage:');
  console.log('  node systems/sensory/causal_vs_correlation_signal_scorer.js run [YYYY-MM-DD] [--strict=1] [--policy=<path>]');
  console.log('  node systems/sensory/causal_vs_correlation_signal_scorer.js status [YYYY-MM-DD] [--policy=<path>]');
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
