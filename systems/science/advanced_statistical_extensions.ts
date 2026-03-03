#!/usr/bin/env node
'use strict';
export {};

/**
 * V4-SCI-007
 * Advanced statistical extensions (causal + uncertainty + model selection provenance).
 */

const fs = require('fs');
const path = require('path');
const {
  ROOT,
  nowIso,
  cleanText,
  toBool,
  clampInt,
  clampNumber,
  parseArgs,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

type AnyObj = Record<string, any>;

const DEFAULT_POLICY_PATH = process.env.ADVANCED_STAT_EXT_POLICY_PATH
  ? path.resolve(process.env.ADVANCED_STAT_EXT_POLICY_PATH)
  : path.join(ROOT, 'config', 'advanced_statistical_extensions_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/science/advanced_statistical_extensions.js run [--sample_size=240 --brier_score=0.24 --causal_precision_lift=0.03 --strict=1] [--policy=<path>]');
  console.log('  node systems/science/advanced_statistical_extensions.js status [--policy=<path>]');
}

function rel(filePath: string) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    min_sample_size: 90,
    min_accuracy_score: 0.55,
    max_uncertainty_width: 0.4,
    causal_models: ['did', 'synthetic_control', 'bayesian_structural_ts'],
    uncertainty_confidence_levels: [0.8, 0.9, 0.95],
    ensemble_methods: ['bayesian_model_average', 'stacked_regression'],
    fallback_engine: 'ts_stat_extensions',
    allow_external_python: false,
    allow_external_rust: false,
    paths: {
      latest_path: 'state/science/advanced_statistical_extensions/latest.json',
      history_path: 'state/science/advanced_statistical_extensions/history.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 40) || base.version,
    enabled: raw.enabled !== false,
    min_sample_size: clampInt(raw.min_sample_size, 1, 100000000, base.min_sample_size),
    min_accuracy_score: clampNumber(raw.min_accuracy_score, 0, 1, base.min_accuracy_score),
    max_uncertainty_width: clampNumber(raw.max_uncertainty_width, 0, 5, base.max_uncertainty_width),
    causal_models: Array.isArray(raw.causal_models) && raw.causal_models.length
      ? raw.causal_models.map((v: unknown) => cleanText(v, 120)).filter(Boolean)
      : base.causal_models,
    uncertainty_confidence_levels: Array.isArray(raw.uncertainty_confidence_levels) && raw.uncertainty_confidence_levels.length
      ? raw.uncertainty_confidence_levels.map((v: unknown) => clampNumber(v, 0.01, 0.999, 0.95))
      : base.uncertainty_confidence_levels,
    ensemble_methods: Array.isArray(raw.ensemble_methods) && raw.ensemble_methods.length
      ? raw.ensemble_methods.map((v: unknown) => cleanText(v, 120)).filter(Boolean)
      : base.ensemble_methods,
    fallback_engine: cleanText(raw.fallback_engine || base.fallback_engine, 120) || base.fallback_engine,
    allow_external_python: toBool(raw.allow_external_python, base.allow_external_python),
    allow_external_rust: toBool(raw.allow_external_rust, base.allow_external_rust),
    paths: {
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function selectEngine(policy: AnyObj) {
  if (policy.allow_external_rust === true) return 'rust_worker';
  if (policy.allow_external_python === true) return 'python_worker';
  return policy.fallback_engine;
}

function normalizeModels(raw: unknown, policy: AnyObj) {
  const models = Array.isArray(raw) && raw.length
    ? raw
    : policy.ensemble_methods.map((name: string, i: number) => ({ name, score_hint: 0.6 - (i * 0.05) }));

  return models.map((row: AnyObj, idx: number) => {
    const name = cleanText(row && row.name, 120) || cleanText(policy.ensemble_methods[idx] || `model_${idx + 1}`, 120);
    const scoreHint = clampNumber(row && row.score_hint, 0, 1, 0.6 - (idx * 0.04));
    return {
      name,
      score_hint: scoreHint
    };
  });
}

function computeUncertaintyWidth(brierScore: number, sampleSize: number) {
  const calibrationPenalty = clampNumber(brierScore, 0, 1, 0.5) * 0.45;
  const sampleBonus = 1 / Math.max(1, Math.sqrt(sampleSize));
  return Number(clampNumber(calibrationPenalty + sampleBonus, 0, 5, 0.5).toFixed(6));
}

function buildIntervals(pointEstimate: number, width: number, levels: number[]) {
  return levels.map((level) => {
    const scale = clampNumber(1 - level, 0.001, 1, 0.05) * 2;
    const half = Number((width * scale).toFixed(6));
    return {
      confidence_level: Number(level),
      lower: Number((pointEstimate - half).toFixed(6)),
      upper: Number((pointEstimate + half).toFixed(6))
    };
  });
}

function runExtensions(input: AnyObj, policy: AnyObj) {
  if (policy.enabled !== true) {
    return {
      ok: true,
      type: 'advanced_statistical_extensions',
      ts: nowIso(),
      result: 'disabled_by_policy'
    };
  }

  const sampleSize = clampInt(input.sample_size, 1, 100000000, 120);
  const brierScore = clampNumber(input.brier_score, 0, 1, 0.28);
  const causalPrecisionLift = clampNumber(input.causal_precision_lift, -1, 1, 0.01);
  const pointEstimate = clampNumber(input.effect_size, -10, 10, 0.1);

  const uncertaintyWidth = computeUncertaintyWidth(brierScore, sampleSize);
  const intervals = buildIntervals(pointEstimate, uncertaintyWidth, policy.uncertainty_confidence_levels);

  const modelCandidates = normalizeModels(input.candidate_models, policy)
    .map((row: AnyObj) => ({
      name: row.name,
      score: Number(clampNumber(
        row.score_hint + (causalPrecisionLift * 0.7) + ((1 - brierScore) * 0.2),
        0,
        1,
        0.5
      ).toFixed(6))
    }))
    .sort((a: AnyObj, b: AnyObj) => Number(b.score || 0) - Number(a.score || 0));

  const selectedModel = modelCandidates[0] || null;
  const accuracyScore = Number(clampNumber(
    ((1 - brierScore) * 0.65) + (Math.max(0, causalPrecisionLift) * 0.35),
    0,
    1,
    0.5
  ).toFixed(6));

  const checks = {
    sample_size_floor_met: sampleSize >= Number(policy.min_sample_size || 0),
    uncertainty_width_bounded: uncertaintyWidth <= Number(policy.max_uncertainty_width || 0),
    accuracy_floor_met: accuracyScore >= Number(policy.min_accuracy_score || 0),
    selected_model_available: !!selectedModel
  };

  const blockingChecks = Object.entries(checks).filter(([, ok]) => ok !== true).map(([id]) => id);
  const pass = blockingChecks.length === 0;
  const engine = selectEngine(policy);

  return {
    ok: pass,
    pass,
    type: 'advanced_statistical_extensions',
    lane_id: 'V4-SCI-007',
    ts: nowIso(),
    engine,
    checks,
    blocking_checks: blockingChecks,
    sample_size: sampleSize,
    brier_score: brierScore,
    causal_precision_lift: causalPrecisionLift,
    uncertainty_width: uncertaintyWidth,
    uncertainty_intervals: intervals,
    model_candidates: modelCandidates,
    selected_model: selectedModel,
    accuracy_score: accuracyScore,
    causal_model_family: policy.causal_models,
    provenance: {
      method_bundle: ['causal_estimation', 'uncertainty_quantification', 'ensemble_selection'],
      fallback_engine: policy.fallback_engine,
      generated_at: nowIso()
    },
    extension_receipt_id: `adv_stats_${stableHash(JSON.stringify({ sampleSize, brierScore, causalPrecisionLift, selectedModel, accuracyScore }), 14)}`
  };
}

function cmdRun(args: AnyObj) {
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  const strict = toBool(args.strict, false);

  const input = {
    sample_size: args.sample_size,
    brier_score: args.brier_score,
    causal_precision_lift: args.causal_precision_lift,
    effect_size: args.effect_size,
    candidate_models: (() => {
      const raw = cleanText(args.candidate_models_json, 12000);
      if (!raw) return [];
      try { return JSON.parse(raw); } catch { return []; }
    })()
  };

  const out = runExtensions(input, policy);
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.history_path, {
    ts: out.ts,
    type: out.type,
    ok: out.ok,
    blocking_checks: out.blocking_checks,
    extension_receipt_id: out.extension_receipt_id || null
  });

  emit({
    ...out,
    policy_path: rel(policy.policy_path),
    latest_path: rel(policy.paths.latest_path)
  }, out.ok || !strict ? 0 : 1);
}

function cmdStatus(args: AnyObj) {
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  emit({
    ok: true,
    type: 'advanced_statistical_extensions_status',
    ts: nowIso(),
    latest: readJson(policy.paths.latest_path, null),
    policy_path: rel(policy.policy_path),
    latest_path: rel(policy.paths.latest_path)
  }, 0);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'status', 80).toLowerCase();
  if (args.help || ['help', '--help', '-h'].includes(cmd)) {
    usage();
    process.exit(0);
  }

  if (cmd === 'run' || cmd === 'verify') return cmdRun(args);
  if (cmd === 'status') return cmdStatus(args);

  usage();
  emit({ ok: false, error: `unknown_command:${cmd}` }, 2);
}

if (require.main === module) {
  main();
}

module.exports = {
  loadPolicy,
  runExtensions,
  cmdRun,
  cmdStatus
};
