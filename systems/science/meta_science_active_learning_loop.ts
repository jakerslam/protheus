#!/usr/bin/env node
'use strict';
export {};

/**
 * V4-SCI-006
 * Meta-science + active-learning loop.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

type AnyObj = Record<string, any>;

const ROOT = process.env.META_SCIENCE_ROOT
  ? path.resolve(process.env.META_SCIENCE_ROOT)
  : path.resolve(__dirname, '..', '..');

const DEFAULT_POLICY_PATH = process.env.META_SCIENCE_POLICY_PATH
  ? path.resolve(process.env.META_SCIENCE_POLICY_PATH)
  : path.join(ROOT, 'config', 'meta_science_active_learning_policy.json');

function nowIso() {
  return new Date().toISOString();
}

function cleanText(v: unknown, maxLen = 320) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function parseArgs(argv: string[]) {
  const out: AnyObj = { _: [] };
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

function parseJsonArg(raw: unknown, fallback: any = null) {
  const txt = String(raw == null ? '' : raw).trim();
  if (!txt) return fallback;
  try { return JSON.parse(txt); } catch { return fallback; }
}

function loadJsonl(filePath: string) {
  if (!fs.existsSync(filePath)) return [];
  return String(fs.readFileSync(filePath, 'utf8') || '')
    .split('\n')
    .filter(Boolean)
    .map((line) => {
      try { return JSON.parse(line); } catch { return null; }
    })
    .filter(Boolean);
}

function writeJsonAtomic(filePath: string, value: AnyObj) {
  ensureDir(path.dirname(filePath));
  const tmp = `${filePath}.tmp-${Date.now()}-${process.pid}`;
  fs.writeFileSync(tmp, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
  fs.renameSync(tmp, filePath);
}

function appendJsonl(filePath: string, row: AnyObj) {
  ensureDir(path.dirname(filePath));
  fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`, 'utf8');
}

function resolvePath(raw: unknown, fallbackRel: string) {
  const txt = cleanText(raw, 520);
  if (!txt) return path.join(ROOT, fallbackRel);
  return path.isAbsolute(txt) ? txt : path.join(ROOT, txt);
}

function rel(filePath: string) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function stableHash(v: unknown, len = 18) {
  return crypto.createHash('sha256').update(String(v == null ? '' : v), 'utf8').digest('hex').slice(0, len);
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    strict_contracts: false,
    quality_thresholds: {
      min_calibration_score: 0.65,
      max_bias_risk: 0.35,
      min_method_effectiveness: 0.55
    },
    active_learning: {
      enabled: true,
      top_k_requests: 3,
      min_uncertainty: 0.55,
      min_impact: 0.25
    },
    primitive_proposals: {
      enabled: true,
      max_candidates: 3
    },
    paths: {
      active_learning_queue_path: 'state/sensory/analysis/active_learning/queue.jsonl',
      scientific_mode_latest_path: 'state/science/scientific_mode_v4/latest.json',
      latest_path: 'state/science/meta_science/latest.json',
      history_path: 'state/science/meta_science/history.jsonl',
      requests_path: 'state/science/meta_science/active_learning_requests.json',
      proposals_path: 'state/science/meta_science/primitive_candidates.json'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const base = defaultPolicy();
  const raw = readJson(policyPath, {});
  const quality = raw.quality_thresholds && typeof raw.quality_thresholds === 'object' ? raw.quality_thresholds : {};
  const al = raw.active_learning && typeof raw.active_learning === 'object' ? raw.active_learning : {};
  const proposals = raw.primitive_proposals && typeof raw.primitive_proposals === 'object' ? raw.primitive_proposals : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 40) || base.version,
    enabled: raw.enabled !== false,
    strict_contracts: raw.strict_contracts === true,
    quality_thresholds: {
      min_calibration_score: clampNumber(quality.min_calibration_score, 0, 1, base.quality_thresholds.min_calibration_score),
      max_bias_risk: clampNumber(quality.max_bias_risk, 0, 1, base.quality_thresholds.max_bias_risk),
      min_method_effectiveness: clampNumber(quality.min_method_effectiveness, 0, 1, base.quality_thresholds.min_method_effectiveness)
    },
    active_learning: {
      enabled: al.enabled !== false,
      top_k_requests: Math.max(1, Math.floor(Number(al.top_k_requests || base.active_learning.top_k_requests))),
      min_uncertainty: clampNumber(al.min_uncertainty, 0, 1, base.active_learning.min_uncertainty),
      min_impact: clampNumber(al.min_impact, 0, 1, base.active_learning.min_impact)
    },
    primitive_proposals: {
      enabled: proposals.enabled !== false,
      max_candidates: Math.max(1, Math.floor(Number(proposals.max_candidates || base.primitive_proposals.max_candidates)))
    },
    paths: {
      active_learning_queue_path: resolvePath(paths.active_learning_queue_path, base.paths.active_learning_queue_path),
      scientific_mode_latest_path: resolvePath(paths.scientific_mode_latest_path, base.paths.scientific_mode_latest_path),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path),
      requests_path: resolvePath(paths.requests_path, base.paths.requests_path),
      proposals_path: resolvePath(paths.proposals_path, base.paths.proposals_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function normalizeUncertaintyCase(row: AnyObj, idx: number) {
  return {
    case_id: cleanText(row.case_id || row.id || '', 100) || `al_case_${idx + 1}`,
    topic: cleanText(row.topic || row.signal_topic || row.objective || 'unspecified', 200),
    uncertainty_score: clampNumber(row.uncertainty_score ?? row.uncertainty ?? 0.5, 0, 1, 0.5),
    impact_score: clampNumber(row.impact_score ?? row.impact ?? row.priority ?? 0.5, 0, 1, 0.5),
    evidence_gap: cleanText(row.evidence_gap || row.reason || 'insufficient_disconfirming_evidence', 260)
  };
}

function rankActiveLearningRequests(rows: AnyObj[], policy: AnyObj) {
  const normalized = rows.map((row, idx) => normalizeUncertaintyCase(row, idx));
  const filtered = normalized.filter((row) => (
    row.uncertainty_score >= Number(policy.active_learning.min_uncertainty || 0)
    && row.impact_score >= Number(policy.active_learning.min_impact || 0)
  ));
  const ranked = filtered.map((row) => {
    const requestScore = Number(((row.uncertainty_score * 0.7) + (row.impact_score * 0.3)).toFixed(6));
    return {
      request_id: `alr_${stableHash(`${row.case_id}|${row.topic}|${requestScore}`, 12)}`,
      ...row,
      request_score: requestScore,
      request_kind: 'collect_disconfirming_signal'
    };
  });
  ranked.sort((a, b) => {
    if (b.request_score !== a.request_score) return b.request_score - a.request_score;
    return String(a.case_id).localeCompare(String(b.case_id));
  });
  return ranked.slice(0, Number(policy.active_learning.top_k_requests || 1));
}

function buildPrimitiveCandidates(metrics: AnyObj, policy: AnyObj) {
  if (policy.primitive_proposals.enabled !== true) return [];
  const candidates: AnyObj[] = [];
  if (metrics.calibration_score < Number(policy.quality_thresholds.min_calibration_score || 0)) {
    candidates.push({
      candidate_id: `primitive_${stableHash('calibration_replay', 10)}`,
      primitive: 'calibration_replay_primitive',
      severity: 'high',
      trigger: 'calibration_below_threshold',
      rationale: 'Brier-linked calibration score is below policy floor.'
    });
  }
  if (metrics.bias_risk > Number(policy.quality_thresholds.max_bias_risk || 1)) {
    candidates.push({
      candidate_id: `primitive_${stableHash('counterfactual_debias', 10)}`,
      primitive: 'counterfactual_debias_primitive',
      severity: 'medium',
      trigger: 'bias_risk_above_threshold',
      rationale: 'Bias-risk estimate exceeded allowed contract.'
    });
  }
  if (metrics.method_effectiveness < Number(policy.quality_thresholds.min_method_effectiveness || 0)) {
    candidates.push({
      candidate_id: `primitive_${stableHash('experiment_design_optimizer', 10)}`,
      primitive: 'experiment_design_optimizer_primitive',
      severity: 'medium',
      trigger: 'method_effectiveness_below_threshold',
      rationale: 'Method effectiveness is below required floor.'
    });
  }
  return candidates.slice(0, Number(policy.primitive_proposals.max_candidates || 1));
}

function computeMetrics(input: AnyObj, policy: AnyObj) {
  const brier = clampNumber(input.brier_score, 0, 1, 0.35);
  const calibrationScore = Number((1 - brier).toFixed(6));
  const biasRisk = clampNumber(input.bias_risk, 0, 1, 0.2);
  const methodEffectiveness = clampNumber(input.method_effectiveness, 0, 1, 0.6);

  const quality = {
    calibration_pass: calibrationScore >= Number(policy.quality_thresholds.min_calibration_score || 0),
    bias_pass: biasRisk <= Number(policy.quality_thresholds.max_bias_risk || 1),
    method_pass: methodEffectiveness >= Number(policy.quality_thresholds.min_method_effectiveness || 0)
  };

  return {
    brier_score: brier,
    calibration_score: calibrationScore,
    bias_risk: biasRisk,
    method_effectiveness: methodEffectiveness,
    quality,
    quality_pass: quality.calibration_pass && quality.bias_pass && quality.method_pass
  };
}

function loadFallbackInput(policy: AnyObj) {
  const latest = readJson(policy.paths.scientific_mode_latest_path, {});
  const loop = latest && latest.loop && typeof latest.loop === 'object' ? latest.loop : {};
  const forge = latest && latest.forge && typeof latest.forge === 'object' ? latest.forge : {};
  const top = forge.top_hypothesis && typeof forge.top_hypothesis === 'object' ? forge.top_hypothesis : {};
  return {
    brier_score: Number(loop.brier_score || latest.brier_score || 0.35),
    bias_risk: Number(latest.bias_risk || 0.2),
    method_effectiveness: Number(top.score || latest.method_effectiveness || 0.6)
  };
}

function runMetaScience(inputRaw: AnyObj, policy: AnyObj, options: AnyObj = {}) {
  if (policy.enabled !== true) {
    return {
      ok: true,
      type: 'meta_science_active_learning_loop',
      ts: nowIso(),
      result: 'disabled_by_policy',
      rollback_hint: 'meta_loop_disabled_core_science_unmodified'
    };
  }

  const fallback = loadFallbackInput(policy);
  const input = {
    brier_score: Number.isFinite(Number(inputRaw.brier_score)) ? Number(inputRaw.brier_score) : fallback.brier_score,
    bias_risk: Number.isFinite(Number(inputRaw.bias_risk)) ? Number(inputRaw.bias_risk) : fallback.bias_risk,
    method_effectiveness: Number.isFinite(Number(inputRaw.method_effectiveness)) ? Number(inputRaw.method_effectiveness) : fallback.method_effectiveness
  };

  const queueRows = Array.isArray(inputRaw.uncertainty_cases)
    ? inputRaw.uncertainty_cases
    : loadJsonl(policy.paths.active_learning_queue_path);

  const metrics = computeMetrics(input, policy);
  const activeRequests = policy.active_learning.enabled === true
    ? rankActiveLearningRequests(queueRows, policy)
    : [];
  const primitiveCandidates = buildPrimitiveCandidates(metrics, policy);

  const payload = {
    ok: policy.strict_contracts === true ? metrics.quality_pass : true,
    ts: nowIso(),
    type: 'meta_science_active_learning_loop',
    metrics,
    active_learning_requests: activeRequests,
    primitive_candidates: primitiveCandidates,
    governed_suggestions_only: true,
    audit_receipt_id: `meta_sci_${stableHash(JSON.stringify({ metrics, activeRequests, primitiveCandidates }), 14)}`,
    rollback_hint: 'disable_meta_science_loop_preserves_scientific_core'
  };

  if (options.persist !== false) {
    writeJsonAtomic(policy.paths.latest_path, payload);
    writeJsonAtomic(policy.paths.requests_path, {
      ts: payload.ts,
      request_count: activeRequests.length,
      requests: activeRequests
    });
    writeJsonAtomic(policy.paths.proposals_path, {
      ts: payload.ts,
      candidate_count: primitiveCandidates.length,
      candidates: primitiveCandidates
    });
    appendJsonl(policy.paths.history_path, {
      ts: payload.ts,
      type: payload.type,
      ok: payload.ok,
      quality_pass: metrics.quality_pass,
      request_count: activeRequests.length,
      candidate_count: primitiveCandidates.length,
      audit_receipt_id: payload.audit_receipt_id
    });
  }

  return payload;
}

function cmdRun(args: AnyObj) {
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  const uncertaintyCases = parseJsonArg(args['uncertainty-json'] || args.uncertainty_json, null);
  const out = runMetaScience({
    brier_score: args.brier_score ?? args.brier,
    bias_risk: args.bias_risk,
    method_effectiveness: args.method_effectiveness,
    uncertainty_cases: Array.isArray(uncertaintyCases) ? uncertaintyCases : undefined
  }, policy);
  return {
    ...out,
    policy_path: rel(policy.policy_path),
    output_paths: {
      latest_path: rel(policy.paths.latest_path),
      history_path: rel(policy.paths.history_path),
      requests_path: rel(policy.paths.requests_path),
      proposals_path: rel(policy.paths.proposals_path)
    }
  };
}

function cmdStatus(args: AnyObj) {
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  return {
    ok: true,
    ts: nowIso(),
    type: 'meta_science_active_learning_loop_status',
    latest: readJson(policy.paths.latest_path, null),
    latest_path: rel(policy.paths.latest_path),
    policy_path: rel(policy.policy_path)
  };
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/science/meta_science_active_learning_loop.js run [--brier=0.24 --bias_risk=0.12 --method_effectiveness=0.7 --uncertainty-json="[]"] [--policy=<path>]');
  console.log('  node systems/science/meta_science_active_learning_loop.js status [--policy=<path>]');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || '', 80).toLowerCase();
  if (!cmd || cmd === 'help' || cmd === '--help' || cmd === '-h' || args.help) {
    usage();
    process.exit(0);
  }

  try {
    const out = cmd === 'run'
      ? cmdRun(args)
      : cmd === 'status'
        ? cmdStatus(args)
        : null;
    if (!out) {
      usage();
      process.exit(2);
    }
    process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
    if (cmd === 'run' && out.ok !== true) process.exit(1);
  } catch (err: any) {
    process.stdout.write(`${JSON.stringify({ ok: false, error: cleanText(err && err.message ? err.message : err, 420) }, null, 2)}\n`);
    process.exit(1);
  }
}

if (require.main === module) {
  main();
}

module.exports = {
  loadPolicy,
  runMetaScience,
  computeMetrics,
  rankActiveLearningRequests,
  buildPrimitiveCandidates,
  cmdRun,
  cmdStatus
};
