#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-091
 * Value-of-information collection planner.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const ROOT = path.resolve(__dirname, '..', '..');
const POLICY_PATH = process.env.VALUE_INFORMATION_PLANNER_POLICY_PATH
  ? path.resolve(process.env.VALUE_INFORMATION_PLANNER_POLICY_PATH)
  : path.join(ROOT, 'config', 'value_of_information_collection_planner_policy.json');

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
    max_actions: 12,
    min_expected_information_gain: 0.08,
    objective_weights: {
      T1_make_jay_billionaire_v1: 1,
      T1_generational_wealth_v1: 0.9
    },
    uncertainty_weights: {
      abstain_count: 0.6,
      unresolved_abstain_rate: 0.4
    },
    paths: {
      abstain_dir: 'state/sensory/analysis/abstain_uncertainty',
      chain_mapper_dir: 'state/sensory/analysis/objective_chain_mapper',
      output_dir: 'state/sensory/analysis/value_information_planner',
      latest_path: 'state/sensory/analysis/value_information_planner/latest.json',
      receipts_path: 'state/sensory/analysis/value_information_planner/receipts.jsonl'
    }
  };
}

function loadPolicy(policyPath = POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  const objectiveWeights = raw.objective_weights && typeof raw.objective_weights === 'object'
    ? raw.objective_weights
    : base.objective_weights;
  const uncertaintyWeights = raw.uncertainty_weights && typeof raw.uncertainty_weights === 'object'
    ? raw.uncertainty_weights
    : base.uncertainty_weights;
  return {
    version: cleanText(raw.version || base.version, 32) || base.version,
    enabled: raw.enabled !== false,
    max_actions: clampNumber(raw.max_actions, 1, 200, base.max_actions),
    min_expected_information_gain: clampNumber(raw.min_expected_information_gain, 0, 1, base.min_expected_information_gain),
    objective_weights: objectiveWeights,
    uncertainty_weights: {
      abstain_count: clampNumber(uncertaintyWeights.abstain_count, 0, 1, base.uncertainty_weights.abstain_count),
      unresolved_abstain_rate: clampNumber(uncertaintyWeights.unresolved_abstain_rate, 0, 1, base.uncertainty_weights.unresolved_abstain_rate)
    },
    paths: {
      abstain_dir: resolvePath(paths.abstain_dir, base.paths.abstain_dir),
      chain_mapper_dir: resolvePath(paths.chain_mapper_dir, base.paths.chain_mapper_dir),
      output_dir: resolvePath(paths.output_dir, base.paths.output_dir),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function loadAbstain(policy: Record<string, any>, dateStr: string) {
  const fp = path.join(policy.paths.abstain_dir, `${dateStr}.json`);
  const src = readJson(fp, null);
  const abstained = src && Array.isArray(src.abstained) ? src.abstained : [];
  return {
    file_path: fp,
    rows: abstained.filter((row: any) => row && typeof row === 'object')
  };
}

function loadChains(policy: Record<string, any>, dateStr: string) {
  const fp = path.join(policy.paths.chain_mapper_dir, `${dateStr}.json`);
  const src = readJson(fp, null);
  const chains = src && Array.isArray(src.chains) ? src.chains : [];
  return {
    file_path: fp,
    rows: chains.filter((row: any) => row && typeof row === 'object')
  };
}

function run(dateStr: string, policy: Record<string, any>, strict = false) {
  const abstainSrc = loadAbstain(policy, dateStr);
  const chainSrc = loadChains(policy, dateStr);

  const topicStats = new Map();

  for (const row of abstainSrc.rows) {
    const topic = normalizeToken(row && row.topic || 'unknown_topic', 120) || 'unknown_topic';
    const slot = topicStats.get(topic) || {
      topic,
      abstain_count: 0,
      unresolved_abstain_count: 0,
      objectives: new Map(),
      max_path_confidence: 0
    };
    slot.abstain_count += 1;
    if (row && row.resolved !== true) slot.unresolved_abstain_count += 1;
    topicStats.set(topic, slot);
  }

  for (const row of chainSrc.rows) {
    const topic = normalizeToken(row && row.topic || 'unknown_topic', 120) || 'unknown_topic';
    const objectiveId = cleanText(row && row.objective_id || 'global', 120) || 'global';
    const slot = topicStats.get(topic) || {
      topic,
      abstain_count: 0,
      unresolved_abstain_count: 0,
      objectives: new Map(),
      max_path_confidence: 0
    };
    const pathConfidence = clampNumber(row && row.path_confidence, 0, 1, 0);
    const objectiveMax = clampNumber(slot.objectives.get(objectiveId), 0, 1, 0);
    slot.objectives.set(objectiveId, Math.max(objectiveMax, pathConfidence));
    slot.max_path_confidence = Math.max(Number(slot.max_path_confidence || 0), pathConfidence);
    topicStats.set(topic, slot);
  }

  const plans = [];

  for (const slot of topicStats.values()) {
    const unresolvedRate = Number(slot.abstain_count || 0) > 0
      ? Number(slot.unresolved_abstain_count || 0) / Number(slot.abstain_count || 1)
      : 0;

    const uncertaintyScore = clampNumber(
      (Math.min(1, Number(slot.abstain_count || 0) / 6) * Number(policy.uncertainty_weights.abstain_count || 0.6))
      + (unresolvedRate * Number(policy.uncertainty_weights.unresolved_abstain_rate || 0.4)),
      0,
      1,
      0
    );

    for (const [objectiveId, objectiveConfidenceRaw] of slot.objectives.entries()) {
      const objectiveWeight = clampNumber(policy.objective_weights && policy.objective_weights[objectiveId], 0, 2, 0.7);
      const objectiveConfidence = clampNumber(objectiveConfidenceRaw, 0, 1, 0);
      const expectedGain = clampNumber(uncertaintyScore * objectiveConfidence * objectiveWeight, 0, 1, 0);
      if (expectedGain < Number(policy.min_expected_information_gain || 0.08)) continue;

      const predictedReduction = clampNumber(expectedGain * 0.65, 0, 1, 0);
      plans.push({
        plan_id: `voi_${stableHash(`${dateStr}|${slot.topic}|${objectiveId}|${expectedGain}`, 20)}`,
        topic: slot.topic,
        objective_id: objectiveId,
        uncertainty_score: Number(uncertaintyScore.toFixed(6)),
        objective_confidence: Number(objectiveConfidence.toFixed(6)),
        objective_weight: Number(objectiveWeight.toFixed(6)),
        expected_information_gain: Number(expectedGain.toFixed(6)),
        predicted_uncertainty_reduction: Number(predictedReduction.toFixed(6)),
        action: {
          type: 'targeted_collection_probe',
          prompt: `Collect disambiguating evidence for topic=${slot.topic} objective=${objectiveId}`,
          collection_budget_hint: Number(clampNumber(expectedGain * 100, 5, 100, 10).toFixed(2))
        }
      });
    }
  }

  plans.sort((a, b) => Number(b.expected_information_gain || 0) - Number(a.expected_information_gain || 0));
  const actions = plans.slice(0, Number(policy.max_actions || 12));

  const out = {
    ok: true,
    type: 'value_of_information_collection_planner',
    ts: nowIso(),
    date: dateStr,
    source_paths: {
      abstain: abstainSrc.file_path,
      chain_mapper: chainSrc.file_path
    },
    plan_count: actions.length,
    total_candidates: plans.length,
    actions
  };

  ensureDir(policy.paths.output_dir);
  writeJsonAtomic(path.join(policy.paths.output_dir, `${dateStr}.json`), out);
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, {
    ts: nowIso(),
    type: 'value_information_planner_receipt',
    date: dateStr,
    plan_count: actions.length,
    top_plan_id: actions[0] ? actions[0].plan_id : null,
    top_expected_information_gain: actions[0] ? actions[0].expected_information_gain : 0
  });

  if (strict && actions.length === 0) {
    process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
    process.exit(2);
  }
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
}

function status(policy: Record<string, any>, dateStr: string) {
  const fp = path.join(policy.paths.output_dir, `${dateStr}.json`);
  const payload = readJson(fp, {
    ok: true,
    type: 'value_of_information_collection_planner_status',
    date: dateStr,
    plan_count: 0
  });
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
}

function usageAndExit(code = 0) {
  console.log('Usage:');
  console.log('  node systems/sensory/value_of_information_collection_planner.js run [YYYY-MM-DD] [--strict=1] [--policy=<path>]');
  console.log('  node systems/sensory/value_of_information_collection_planner.js status [YYYY-MM-DD] [--policy=<path>]');
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
