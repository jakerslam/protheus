#!/usr/bin/env node
'use strict';
export {};

/**
 * V4-SCI-008
 * Enhanced reasoning mirror with calibration, uncertainty charting,
 * disconfirming-evidence targets, and one-click experiment routing.
 */

const path = require('path');
const {
  ROOT,
  nowIso,
  cleanText,
  toBool,
  clampNumber,
  clampInt,
  parseArgs,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash
} = require('../../lib/queued_backlog_runtime');
const mirror = require('./reasoning_mirror.js');
const scheduler = require('./experiment_scheduler.js');

type AnyObj = Record<string, any>;

const DEFAULT_POLICY_PATH = process.env.ENHANCED_REASONING_MIRROR_POLICY_PATH
  ? path.resolve(process.env.ENHANCED_REASONING_MIRROR_POLICY_PATH)
  : path.join(ROOT, 'config', 'enhanced_reasoning_mirror_policy.json');

function rel(filePath: string) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    scientific_flag_required: true,
    min_calibration_samples: 120,
    calibration_targets: {
      brier_excellent: 0.18,
      brier_good: 0.28,
      max_confidence_gap: 0.12
    },
    uncertainty_levels: [0.5, 0.8, 0.9, 0.95],
    consent_map_path: 'state/science/experiment_scheduler/consent_map.json',
    scheduler_policy_path: 'config/experiment_scheduler_policy.json',
    paths: {
      hypothesis_latest_path: 'state/science/hypothesis_forge/latest.json',
      loop_latest_path: 'state/science/loop/latest.json',
      latest_path: 'state/science/enhanced_reasoning_mirror/latest.json',
      ui_contract_path: 'state/science/enhanced_reasoning_mirror/ui_contract.json',
      history_path: 'state/science/enhanced_reasoning_mirror/history.jsonl',
      route_hypothesis_path: 'state/science/enhanced_reasoning_mirror/route_hypothesis.json',
      routed_history_path: 'state/science/enhanced_reasoning_mirror/routed_history.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const base = defaultPolicy();
  const raw = readJson(policyPath, {});
  const ct = raw.calibration_targets && typeof raw.calibration_targets === 'object' ? raw.calibration_targets : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 40) || base.version,
    enabled: raw.enabled !== false,
    scientific_flag_required: raw.scientific_flag_required !== false,
    min_calibration_samples: clampInt(raw.min_calibration_samples, 10, 10_000_000, base.min_calibration_samples),
    calibration_targets: {
      brier_excellent: clampNumber(ct.brier_excellent, 0, 1, base.calibration_targets.brier_excellent),
      brier_good: clampNumber(ct.brier_good, 0, 1, base.calibration_targets.brier_good),
      max_confidence_gap: clampNumber(ct.max_confidence_gap, 0, 1, base.calibration_targets.max_confidence_gap)
    },
    uncertainty_levels: Array.isArray(raw.uncertainty_levels) && raw.uncertainty_levels.length
      ? raw.uncertainty_levels.map((v: unknown) => clampNumber(v, 0.01, 0.999, 0.9))
      : base.uncertainty_levels,
    consent_map_path: resolvePath(raw.consent_map_path, base.consent_map_path),
    scheduler_policy_path: resolvePath(raw.scheduler_policy_path, base.scheduler_policy_path),
    paths: {
      hypothesis_latest_path: resolvePath(paths.hypothesis_latest_path, base.paths.hypothesis_latest_path),
      loop_latest_path: resolvePath(paths.loop_latest_path, base.paths.loop_latest_path),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      ui_contract_path: resolvePath(paths.ui_contract_path, base.paths.ui_contract_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path),
      route_hypothesis_path: resolvePath(paths.route_hypothesis_path, base.paths.route_hypothesis_path),
      routed_history_path: resolvePath(paths.routed_history_path, base.paths.routed_history_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function calibrationBand(score: number, brier: number, gap: number, sampleSize: number, policy: AnyObj) {
  if (sampleSize < policy.min_calibration_samples) return 'insufficient_data';
  if (brier <= policy.calibration_targets.brier_excellent && gap <= (policy.calibration_targets.max_confidence_gap * 0.7)) {
    return 'well_calibrated';
  }
  if (brier <= policy.calibration_targets.brier_good && gap <= policy.calibration_targets.max_confidence_gap) {
    return 'acceptable';
  }
  if (score >= 0.8 && gap > policy.calibration_targets.max_confidence_gap) return 'overconfident';
  return 'needs_recalibration';
}

function buildUncertaintyChart(confidence: number, brier: number, gap: number, levels: number[]) {
  return levels.map((levelRaw: number) => {
    const level = clampNumber(levelRaw, 0.01, 0.999, 0.9);
    const baseWidth = Math.max(0.02, 1 - level);
    const qualityPenalty = clampNumber((brier * 0.45) + (gap * 0.55), 0, 0.45, 0.12);
    const width = clampNumber(baseWidth + qualityPenalty, 0.02, 0.98, 0.18);
    return {
      level: Number(level.toFixed(3)),
      lower: Number(Math.max(0, confidence - (width / 2)).toFixed(4)),
      upper: Number(Math.min(1, confidence + (width / 2)).toFixed(4)),
      width: Number(width.toFixed(4))
    };
  });
}

function buildDisconfirmingTargets(baseContract: AnyObj) {
  const effectSize = Number(baseContract && baseContract.key_statistical_outputs && baseContract.key_statistical_outputs.effect_size || 0);
  const pValue = Number(baseContract && baseContract.key_statistical_outputs && baseContract.key_statistical_outputs.p_value || 1);
  const sampleSize = Number(baseContract && baseContract.key_statistical_outputs && baseContract.key_statistical_outputs.sample_size || 0);
  return [
    {
      id: 'target_effect_direction_flip',
      question: 'What result would directly falsify the current causal direction?',
      target: 'Observe effect_size <= 0 with equivalent segmentation.',
      current_effect_size: Number(effectSize.toFixed(6))
    },
    {
      id: 'target_significance_decay',
      question: 'What result would break confidence in current significance?',
      target: 'Observe p_value >= 0.10 after holdout or replication.',
      current_p_value: Number(pValue.toFixed(6))
    },
    {
      id: 'target_replication_consistency',
      question: 'What replication evidence would change the current conclusion?',
      target: 'Run independent replication with >=95% of current sample size and compare confidence interval overlap.',
      current_sample_size: sampleSize
    }
  ];
}

function buildRouteCommand(consentMapPath: string, apply = true) {
  const applyFlag = apply ? '--apply=1' : '--apply=0';
  return `node systems/science/enhanced_reasoning_mirror.js route-suggested ${applyFlag} --consent-map-file=${rel(consentMapPath)}`;
}

function renderEnhancedContract(input: AnyObj, policy: AnyObj) {
  if (policy.enabled !== true) {
    return {
      ok: true,
      type: 'enhanced_reasoning_mirror',
      ts: nowIso(),
      result: 'disabled_by_policy'
    };
  }

  const scientificModeEnabled = toBool(input.scientific_mode_v4_enabled, false);
  if (policy.scientific_flag_required === true && scientificModeEnabled !== true) {
    return {
      ok: true,
      type: 'enhanced_reasoning_mirror',
      ts: nowIso(),
      result: 'disabled_by_scientific_flag'
    };
  }

  const baseContract = mirror.buildMirrorContract(input.forge_latest || {}, input.loop_latest || {});
  const confidence = clampNumber(
    baseContract && baseContract.active_hypothesis ? baseContract.active_hypothesis.score : 0.5,
    0,
    1,
    0.5
  );
  const brier = clampNumber(input.brier_score, 0, 1, Number((1 - confidence).toFixed(6)));
  const empiricalAccuracy = clampNumber(input.empirical_accuracy, 0, 1, Number((1 - brier).toFixed(6)));
  const sampleSize = clampInt(
    input.sample_size != null ? input.sample_size : (baseContract && baseContract.key_statistical_outputs ? baseContract.key_statistical_outputs.sample_size : 0),
    0,
    10_000_000,
    0
  );
  const confidenceGap = Number(Math.abs(confidence - empiricalAccuracy).toFixed(6));
  const band = calibrationBand(confidence, brier, confidenceGap, sampleSize, policy);
  const uncertaintyChart = buildUncertaintyChart(confidence, brier, confidenceGap, policy.uncertainty_levels);
  const targets = buildDisconfirmingTargets(baseContract);

  const routeHypothesis = {
    id: cleanText(baseContract && baseContract.active_hypothesis && baseContract.active_hypothesis.id, 120) || `hyp_${stableHash(JSON.stringify(baseContract), 8)}`,
    text: cleanText(baseContract && baseContract.active_hypothesis && baseContract.active_hypothesis.text, 1800),
    score: confidence,
    voi: clampNumber(input.voi, 0, 1, 0.7),
    risk: clampNumber(input.risk, 0, 1, 0.3),
    rank_receipt_id: baseContract && baseContract.receipt_linkage && Array.isArray(baseContract.receipt_linkage.source_receipt_ids)
      ? cleanText(baseContract.receipt_linkage.source_receipt_ids[1], 120) || null
      : null
  };

  const enhancedReceiptId = `enh_mirror_${stableHash(JSON.stringify({
    base: baseContract.receipt_linkage,
    brier,
    empiricalAccuracy,
    band,
    sampleSize
  }), 14)}`;

  return {
    ok: true,
    type: 'enhanced_reasoning_mirror',
    lane_id: 'V4-SCI-008',
    ts: nowIso(),
    scientific_mode_v4_enabled: scientificModeEnabled,
    base_contract: baseContract,
    calibration_metrics: {
      sample_size: sampleSize,
      confidence_score: Number(confidence.toFixed(6)),
      empirical_accuracy: Number(empiricalAccuracy.toFixed(6)),
      brier_score: Number(brier.toFixed(6)),
      confidence_gap: confidenceGap,
      calibration_band: band
    },
    uncertainty_chart: {
      metric: 'confidence_interval_width',
      points: uncertaintyChart
    },
    disconfirming_evidence_targets: targets,
    what_would_change_my_mind: targets.map((row: AnyObj) => row.target),
    suggested_experiment: {
      consent_required: true,
      scheduler_policy_path: rel(policy.scheduler_policy_path),
      consent_map_path: rel(policy.consent_map_path),
      route_hypothesis: routeHypothesis,
      route_command: buildRouteCommand(policy.consent_map_path, true),
      dry_run_command: buildRouteCommand(policy.consent_map_path, false)
    },
    receipt_linkage: {
      source_receipt_ids: (baseContract && baseContract.receipt_linkage && Array.isArray(baseContract.receipt_linkage.source_receipt_ids))
        ? baseContract.receipt_linkage.source_receipt_ids.filter(Boolean)
        : [],
      enhanced_receipt_id: enhancedReceiptId
    }
  };
}

function cmdRender(args: AnyObj) {
  const policy = loadPolicy(args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH);
  const strict = toBool(args.strict, false);
  const hypothesisFile = cleanText(args['hypothesis-file'] || args.hypothesis_file, 520);
  const loopFile = cleanText(args['loop-file'] || args.loop_file, 520);
  const forgeLatest = readJson(
    hypothesisFile ? (path.isAbsolute(hypothesisFile) ? hypothesisFile : path.join(ROOT, hypothesisFile)) : policy.paths.hypothesis_latest_path,
    {}
  );
  const loopLatest = readJson(
    loopFile ? (path.isAbsolute(loopFile) ? loopFile : path.join(ROOT, loopFile)) : policy.paths.loop_latest_path,
    {}
  );

  const out = renderEnhancedContract({
    forge_latest: forgeLatest,
    loop_latest: loopLatest,
    scientific_mode_v4_enabled: args['scientific-mode'] ?? args.scientific_mode ?? args.scientific_mode_v4_enabled,
    brier_score: args.brier_score ?? args['brier-score'],
    empirical_accuracy: args.empirical_accuracy ?? args['empirical-accuracy'],
    sample_size: args.sample_size ?? args['sample-size'],
    voi: args.voi,
    risk: args.risk
  }, policy);

  writeJsonAtomic(policy.paths.ui_contract_path, out);
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.history_path, {
    ts: out.ts,
    type: out.type,
    ok: out.ok,
    result: out.result || 'enhanced_contract_rendered',
    calibration_band: out.calibration_metrics ? out.calibration_metrics.calibration_band : null,
    source_receipt_ids: out.receipt_linkage ? out.receipt_linkage.source_receipt_ids : []
  });

  const payload = {
    ...out,
    policy_path: rel(policy.policy_path),
    latest_path: rel(policy.paths.latest_path),
    ui_contract_path: rel(policy.paths.ui_contract_path)
  };
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
  if (strict && out.ok !== true) process.exit(1);
}

function cmdRouteSuggested(args: AnyObj) {
  const policy = loadPolicy(args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH);
  const strict = toBool(args.strict, false);
  const latest = readJson(policy.paths.latest_path, null);
  if (!latest || latest.ok !== true || !latest.suggested_experiment || !latest.suggested_experiment.route_hypothesis) {
    const fail = {
      ok: false,
      type: 'enhanced_reasoning_mirror_route',
      ts: nowIso(),
      error: 'missing_latest_enhanced_contract'
    };
    process.stdout.write(`${JSON.stringify(fail, null, 2)}\n`);
    if (strict) process.exit(1);
    return;
  }

  const routeHypothesis = latest.suggested_experiment.route_hypothesis;
  const consentMapPathRaw = cleanText(args['consent-map-file'] || args.consent_map_file, 520);
  const consentMapPath = consentMapPathRaw
    ? (path.isAbsolute(consentMapPathRaw) ? consentMapPathRaw : path.join(ROOT, consentMapPathRaw))
    : policy.consent_map_path;
  const apply = toBool(args.apply, false);

  writeJsonAtomic(policy.paths.route_hypothesis_path, [routeHypothesis]);
  const scheduleOut = scheduler.cmdSchedule({
    policy: policy.scheduler_policy_path,
    'hypotheses-file': policy.paths.route_hypothesis_path,
    'consent-map-file': consentMapPath,
    apply,
    'now-iso': cleanText(args['now-iso'] || args.now_iso, 80) || undefined
  });

  const out = {
    ok: scheduleOut && scheduleOut.ok === true,
    type: 'enhanced_reasoning_mirror_route',
    lane_id: 'V4-SCI-008',
    ts: nowIso(),
    apply,
    hypothesis_id: routeHypothesis.id,
    scheduler_result: scheduleOut,
    source_receipt_ids: latest.receipt_linkage ? latest.receipt_linkage.source_receipt_ids : [],
    enhanced_receipt_id: latest.receipt_linkage ? latest.receipt_linkage.enhanced_receipt_id : null,
    route_receipt_id: `enh_route_${stableHash(JSON.stringify({
      hypothesis_id: routeHypothesis.id,
      schedule: scheduleOut && scheduleOut.ts,
      apply
    }), 14)}`
  };

  appendJsonl(policy.paths.routed_history_path, {
    ts: out.ts,
    type: out.type,
    ok: out.ok,
    apply,
    hypothesis_id: out.hypothesis_id,
    route_receipt_id: out.route_receipt_id
  });

  process.stdout.write(`${JSON.stringify({
    ...out,
    policy_path: rel(policy.policy_path),
    route_hypothesis_path: rel(policy.paths.route_hypothesis_path)
  }, null, 2)}\n`);
  if (strict && out.ok !== true) process.exit(1);
}

function cmdStatus(args: AnyObj) {
  const policy = loadPolicy(args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH);
  const out = {
    ok: true,
    type: 'enhanced_reasoning_mirror_status',
    ts: nowIso(),
    latest: readJson(policy.paths.latest_path, null),
    policy_path: rel(policy.policy_path),
    latest_path: rel(policy.paths.latest_path),
    ui_contract_path: rel(policy.paths.ui_contract_path)
  };
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/science/enhanced_reasoning_mirror.js render [--scientific-mode=1] [--brier-score=<n>] [--empirical-accuracy=<n>] [--sample-size=<n>] [--strict=1] [--policy=<path>]');
  console.log('  node systems/science/enhanced_reasoning_mirror.js route-suggested [--consent-map-file=<path>] [--apply=1|0] [--strict=1] [--policy=<path>]');
  console.log('  node systems/science/enhanced_reasoning_mirror.js status [--policy=<path>]');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'status', 80).toLowerCase();
  if (args.help || ['help', '--help', '-h'].includes(cmd)) {
    usage();
    process.exit(0);
  }
  if (cmd === 'render' || cmd === 'run') return cmdRender(args);
  if (cmd === 'route-suggested') return cmdRouteSuggested(args);
  if (cmd === 'status') return cmdStatus(args);
  usage();
  process.stdout.write(`${JSON.stringify({ ok: false, error: `unknown_command:${cmd}` }, null, 2)}\n`);
  process.exit(2);
}

if (require.main === module) {
  main();
}

module.exports = {
  loadPolicy,
  renderEnhancedContract,
  cmdRender,
  cmdRouteSuggested,
  cmdStatus
};
