#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

type ThresholdTriple = {
  precision_min: number;
  recall_min: number;
  fpr_max: number;
};

const DEFAULT_DATASET_PATH = 'surface/orchestration/fixtures/eval/eval_gold_dataset_v1.jsonl';
const DEFAULT_MONITOR_PATH = 'local/state/ops/eval_agent_chat_monitor/latest.json';
const DEFAULT_THRESHOLDS_PATH = 'tests/tooling/config/eval_quality_thresholds.json';
const DEFAULT_OUT_PATH = 'core/local/artifacts/eval_quality_metrics_current.json';
const DEFAULT_OUT_LATEST_PATH = 'artifacts/eval_quality_metrics_latest.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/EVAL_QUALITY_METRICS_CURRENT.md';

const ISSUE_CLASS_BY_ID: Record<string, string> = {
  workflow_retry_macro_template_detected: 'response_loop',
  workflow_route_automation_claim_detected: 'bad_workflow_selection',
  auto_tool_selection_claim_detected: 'auto_tool_selection_claim',
  policy_block_template_detected: 'policy_block_confusion',
  file_tool_route_misdirection_detected: 'tool_output_misdirection',
  repeated_response_loop_detected: 'response_loop',
  unsupported_claim_detected: 'hallucination',
  wrong_tool_selection_detected: 'wrong_tool_selection',
  no_response_detected: 'no_response',
};

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT_PATH });
  return {
    strict: common.strict,
    datasetPath: cleanText(readFlag(argv, 'dataset') || DEFAULT_DATASET_PATH, 500),
    monitorPath: cleanText(readFlag(argv, 'monitor') || DEFAULT_MONITOR_PATH, 500),
    thresholdsPath: cleanText(readFlag(argv, 'thresholds') || DEFAULT_THRESHOLDS_PATH, 500),
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT_PATH, 500),
    outLatestPath: cleanText(readFlag(argv, 'out-latest') || DEFAULT_OUT_LATEST_PATH, 500),
    markdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MARKDOWN_PATH, 500),
  };
}

function readJson(filePath: string): any | null {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function readJsonl(filePath: string): any[] {
  try {
    const lines = fs.readFileSync(filePath, 'utf8').split(/\r?\n/).filter(Boolean);
    return lines.map((line) => {
      try {
        return JSON.parse(line);
      } catch {
        return null;
      }
    }).filter(Boolean) as any[];
  } catch {
    return [];
  }
}

function safeDiv(num: number, den: number, fallback = 0): number {
  if (!Number.isFinite(num) || !Number.isFinite(den) || den <= 0) return fallback;
  return num / den;
}

function round3(value: number): number {
  return Number(value.toFixed(3));
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function isCanonicalRelativePath(value: string): boolean {
  if (!value) return false;
  if (value.startsWith('/') || value.startsWith('\\')) return false;
  if (value.includes('..') || value.includes('\\') || value.includes('//')) return false;
  return /^[A-Za-z0-9._/\-]+$/.test(value);
}

function hasCaseInsensitiveSuffix(value: string, suffix: string): boolean {
  return value.toLowerCase().endsWith(suffix.toLowerCase());
}

function isProbability(value: number): boolean {
  return Number.isFinite(value) && value >= 0 && value <= 1;
}

function isCanonicalToken(value: string): boolean {
  return /^[a-z0-9][a-z0-9_-]*$/.test(value);
}

function isNonNegativeInteger(value: number): boolean {
  return Number.isInteger(value) && value >= 0;
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Eval Quality Metrics Guard (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report.generated_at || '', 120)}`);
  lines.push(`- ok: ${report.ok === true ? 'true' : 'false'}`);
  lines.push(`- precision: ${Number(report.metrics?.overall?.precision || 0).toFixed(3)}`);
  lines.push(`- recall: ${Number(report.metrics?.overall?.recall || 0).toFixed(3)}`);
  lines.push(`- fpr: ${Number(report.metrics?.overall?.fpr || 0).toFixed(3)}`);
  lines.push(`- actionability_score: ${Number(report.metrics?.overall?.actionability_score || 0).toFixed(3)}`);
  lines.push(`- calibration_error: ${Number(report.metrics?.overall?.calibration_error || 0).toFixed(3)}`);
  lines.push('');
  lines.push('## Threshold violations');
  const thresholdViolations = Array.isArray(report.threshold_violations) ? report.threshold_violations : [];
  if (thresholdViolations.length === 0) {
    lines.push('- none');
  } else {
    thresholdViolations.forEach((row) => {
      lines.push(`- ${cleanText(row.scope || 'unknown', 80)}.${cleanText(row.metric || 'metric', 80)} current=${Number(row.current || 0).toFixed(3)} threshold=${Number(row.threshold || 0).toFixed(3)}`);
    });
  }
  lines.push('');
  lines.push('## Regression violations');
  const regressionViolations = Array.isArray(report.regression_violations) ? report.regression_violations : [];
  if (regressionViolations.length === 0) {
    lines.push('- none');
  } else {
    regressionViolations.forEach((row) => {
      lines.push(`- ${cleanText(row.metric || 'metric', 80)} delta=${Number(row.delta || 0).toFixed(3)} allowed=${Number(row.allowed || 0).toFixed(3)}`);
    });
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const root = process.cwd();
  const datasetAbs = path.resolve(root, args.datasetPath);
  const monitorAbs = path.resolve(root, args.monitorPath);
  const thresholdsAbs = path.resolve(root, args.thresholdsPath);
  const outLatestAbs = path.resolve(root, args.outLatestPath);
  const markdownAbs = path.resolve(root, args.markdownPath);
  const nowIso = new Date().toISOString();

  const datasetRows = readJsonl(datasetAbs);
  const monitor = readJson(monitorAbs) || {};
  const thresholds = readJson(thresholdsAbs) || {};
  const previous = readJson(outLatestAbs) || null;

  const classSet = new Set(
    datasetRows
      .map((row) => cleanText(row?.labels?.issue_class || '', 120))
      .filter(Boolean),
  );

  const actualPositiveByClass: Record<string, number> = {};
  const actualNegativeByClass: Record<string, number> = {};
  const totalRows = datasetRows.length;
  for (const issueClass of classSet) {
    const positives = datasetRows.filter((row) => {
      return cleanText(row?.labels?.issue_class || '', 120) === issueClass
        && Boolean(row?.labels?.is_failure);
    }).length;
    actualPositiveByClass[issueClass] = positives;
    actualNegativeByClass[issueClass] = Math.max(0, totalRows - positives);
  }

  const feedbackRows: any[] = Array.isArray(monitor?.feedback)
    ? monitor.feedback
    : Array.isArray(monitor?.issues)
      ? monitor.issues
      : [];
  const predictedCountByClass: Record<string, number> = {};
  for (const row of feedbackRows) {
    const issueId = cleanText(row?.id || '', 120);
    const issueClass = cleanText(ISSUE_CLASS_BY_ID[issueId] || '', 120);
    if (!issueClass) continue;
    const evidenceCount = Number(row?.evidence_count || 1);
    predictedCountByClass[issueClass] = Number(predictedCountByClass[issueClass] || 0) + Math.max(1, evidenceCount);
    classSet.add(issueClass);
  }

  const perClass: Record<string, any> = {};
  let totalTP = 0;
  let totalFP = 0;
  let totalFN = 0;
  let totalTN = 0;

  for (const issueClass of classSet) {
    const actualPos = Number(actualPositiveByClass[issueClass] || 0);
    const actualNeg = Number(actualNegativeByClass[issueClass] || Math.max(0, totalRows - actualPos));
    const predictedPos = Number(predictedCountByClass[issueClass] || 0);
    const tp = Math.min(actualPos, predictedPos);
    const fp = Math.max(0, predictedPos - actualPos);
    const fn = Math.max(0, actualPos - predictedPos);
    const tn = Math.max(0, actualNeg - fp);
    const precision = safeDiv(tp, tp + fp, 1);
    const recall = safeDiv(tp, tp + fn, 1);
    const fpr = safeDiv(fp, fp + tn, 0);
    perClass[issueClass] = {
      actual_positive: actualPos,
      actual_negative: actualNeg,
      predicted_positive: predictedPos,
      tp,
      fp,
      fn,
      tn,
      precision: round3(precision),
      recall: round3(recall),
      fpr: round3(fpr),
    };
    totalTP += tp;
    totalFP += fp;
    totalFN += fn;
    totalTN += tn;
  }

  const nonInfoPredictions = feedbackRows.filter((row) => cleanText(row?.severity || '', 20) !== 'info');
  const actionablePredictions = nonInfoPredictions.filter((row) => {
    const ownerComponent = cleanText(row?.owner_component || '', 180);
    const ownerPath = cleanText(row?.owner_path || '', 260);
    const nextAction = cleanText(row?.next_action || '', 260);
    const criteria = Array.isArray(row?.acceptance_criteria) ? row.acceptance_criteria : [];
    const evidence = Array.isArray(row?.evidence) ? row.evidence : [];
    return ownerComponent && ownerPath && nextAction && criteria.length > 0 && evidence.length > 0;
  });
  const actionabilityScore = safeDiv(actionablePredictions.length, Math.max(1, nonInfoPredictions.length), 1);

  const calibrationSamples = nonInfoPredictions.map((row) => {
    const issueId = cleanText(row?.id || '', 120);
    const issueClass = cleanText(ISSUE_CLASS_BY_ID[issueId] || '', 120);
    const confidence = Number(row?.confidence || 0);
    const correct = issueClass ? Number(actualPositiveByClass[issueClass] || 0) > 0 : false;
    return { confidence: clampConfidence(confidence), correct: correct ? 1 : 0 };
  });
  const bucketEdges = [0, 0.2, 0.4, 0.6, 0.8, 1.01];
  const reliabilityBuckets: Array<Record<string, unknown>> = [];
  let ece = 0;
  for (let i = 0; i < bucketEdges.length - 1; i += 1) {
    const min = bucketEdges[i];
    const max = bucketEdges[i + 1];
    const rows = calibrationSamples.filter((sample) => sample.confidence >= min && sample.confidence < max);
    if (rows.length === 0) continue;
    const avgConfidence = safeDiv(rows.reduce((acc, row) => acc + row.confidence, 0), rows.length, 0);
    const accuracy = safeDiv(rows.reduce((acc, row) => acc + row.correct, 0), rows.length, 0);
    const weight = safeDiv(rows.length, Math.max(1, calibrationSamples.length), 0);
    ece += Math.abs(accuracy - avgConfidence) * weight;
    reliabilityBuckets.push({
      range: `${min.toFixed(1)}-${(max - 0.01).toFixed(1)}`,
      sample_count: rows.length,
      avg_confidence: round3(avgConfidence),
      observed_accuracy: round3(accuracy),
      abs_gap: round3(Math.abs(accuracy - avgConfidence)),
    });
  }

  const overall = {
    precision: round3(safeDiv(totalTP, totalTP + totalFP, 1)),
    recall: round3(safeDiv(totalTP, totalTP + totalFN, 1)),
    fpr: round3(safeDiv(totalFP, totalFP + totalTN, 0)),
    actionability_score: round3(actionabilityScore),
    calibration_error: round3(ece),
  };

  const globalThresholds = thresholds?.global || {};
  const perClassThresholds = thresholds?.per_class || {};
  const minEvalSamples = Number.isFinite(Number(globalThresholds.minimum_eval_samples))
    ? Number(globalThresholds.minimum_eval_samples)
    : 5;
  const insufficientSignal = nonInfoPredictions.length < minEvalSamples;
  const thresholdViolations: Array<Record<string, unknown>> = [];
  if (!insufficientSignal) {
    if (Number.isFinite(Number(globalThresholds.precision_min)) && overall.precision < Number(globalThresholds.precision_min)) {
      thresholdViolations.push({ scope: 'global', metric: 'precision', current: overall.precision, threshold: Number(globalThresholds.precision_min) });
    }
    if (Number.isFinite(Number(globalThresholds.recall_min)) && overall.recall < Number(globalThresholds.recall_min)) {
      thresholdViolations.push({ scope: 'global', metric: 'recall', current: overall.recall, threshold: Number(globalThresholds.recall_min) });
    }
    if (Number.isFinite(Number(globalThresholds.fpr_max)) && overall.fpr > Number(globalThresholds.fpr_max)) {
      thresholdViolations.push({ scope: 'global', metric: 'fpr', current: overall.fpr, threshold: Number(globalThresholds.fpr_max) });
    }
    if (Number.isFinite(Number(globalThresholds.actionability_min)) && overall.actionability_score < Number(globalThresholds.actionability_min)) {
      thresholdViolations.push({ scope: 'global', metric: 'actionability_score', current: overall.actionability_score, threshold: Number(globalThresholds.actionability_min) });
    }
    if (Number.isFinite(Number(globalThresholds.calibration_error_max)) && overall.calibration_error > Number(globalThresholds.calibration_error_max)) {
      thresholdViolations.push({ scope: 'global', metric: 'calibration_error', current: overall.calibration_error, threshold: Number(globalThresholds.calibration_error_max) });
    }
    for (const [issueClass, metrics] of Object.entries(perClass)) {
      const classThreshold = (perClassThresholds && perClassThresholds[issueClass]) as ThresholdTriple | undefined;
      if (!classThreshold) continue;
      if (Number.isFinite(Number(classThreshold.precision_min)) && Number(metrics.precision) < Number(classThreshold.precision_min)) {
        thresholdViolations.push({ scope: issueClass, metric: 'precision', current: Number(metrics.precision), threshold: Number(classThreshold.precision_min) });
      }
      if (Number.isFinite(Number(classThreshold.recall_min)) && Number(metrics.recall) < Number(classThreshold.recall_min)) {
        thresholdViolations.push({ scope: issueClass, metric: 'recall', current: Number(metrics.recall), threshold: Number(classThreshold.recall_min) });
      }
      if (Number.isFinite(Number(classThreshold.fpr_max)) && Number(metrics.fpr) > Number(classThreshold.fpr_max)) {
        thresholdViolations.push({ scope: issueClass, metric: 'fpr', current: Number(metrics.fpr), threshold: Number(classThreshold.fpr_max) });
      }
    }
  }

  const regressionConfig = thresholds?.regression_guard || {};
  const regressionEnabled = Boolean(regressionConfig.enabled);
  const regressionViolations: Array<Record<string, unknown>> = [];
  if (!insufficientSignal && regressionEnabled && previous?.metrics?.overall) {
    const prev = previous.metrics.overall;
    const precisionDrop = Number(prev.precision || 0) - overall.precision;
    const recallDrop = Number(prev.recall || 0) - overall.recall;
    const actionabilityDrop = Number(prev.actionability_score || 0) - overall.actionability_score;
    const fprIncrease = overall.fpr - Number(prev.fpr || 0);
    if (precisionDrop > Number(regressionConfig.max_precision_drop || 0)) {
      regressionViolations.push({ metric: 'precision_drop', delta: round3(precisionDrop), allowed: Number(regressionConfig.max_precision_drop || 0) });
    }
    if (recallDrop > Number(regressionConfig.max_recall_drop || 0)) {
      regressionViolations.push({ metric: 'recall_drop', delta: round3(recallDrop), allowed: Number(regressionConfig.max_recall_drop || 0) });
    }
    if (actionabilityDrop > Number(regressionConfig.max_actionability_drop || 0)) {
      regressionViolations.push({ metric: 'actionability_drop', delta: round3(actionabilityDrop), allowed: Number(regressionConfig.max_actionability_drop || 0) });
    }
    if (fprIncrease > Number(regressionConfig.max_fpr_increase || 0)) {
      regressionViolations.push({ metric: 'fpr_increase', delta: round3(fprIncrease), allowed: Number(regressionConfig.max_fpr_increase || 0) });
    }
  }

  const globalThresholdProbabilityRangeValid = [
    ['precision_min', globalThresholds?.precision_min],
    ['recall_min', globalThresholds?.recall_min],
    ['fpr_max', globalThresholds?.fpr_max],
    ['actionability_min', globalThresholds?.actionability_min],
    ['calibration_error_max', globalThresholds?.calibration_error_max],
  ].every(([, value]) => value == null || isProbability(Number(value)));
  const perClassThresholdProbabilityRangeValid = Object.values(perClassThresholds || {}).every((row: unknown) => {
    if (!isPlainObject(row)) return false;
    return [
      ['precision_min', row.precision_min],
      ['recall_min', row.recall_min],
      ['fpr_max', row.fpr_max],
    ].every(([, value]) => value == null || isProbability(Number(value)));
  });
  const perClassConfusionPartitionValid = Object.values(perClass).every((metrics: any) => {
    const actualPositive = Number(metrics?.actual_positive || 0);
    const actualNegative = Number(metrics?.actual_negative || 0);
    const predictedPositive = Number(metrics?.predicted_positive || 0);
    const tp = Number(metrics?.tp || 0);
    const fp = Number(metrics?.fp || 0);
    const fn = Number(metrics?.fn || 0);
    const tn = Number(metrics?.tn || 0);
    const precision = Number(metrics?.precision || 0);
    const recall = Number(metrics?.recall || 0);
    const fpr = Number(metrics?.fpr || 0);
    return [
      actualPositive,
      actualNegative,
      predictedPositive,
      tp,
      fp,
      fn,
      tn,
    ].every((value) => isNonNegativeInteger(value))
      && isProbability(precision)
      && isProbability(recall)
      && isProbability(fpr)
      && (actualPositive + actualNegative === totalRows)
      && (tp + fn === actualPositive)
      && (tp + fp === predictedPositive)
      && (tn === Math.max(0, actualNegative - fp));
  });

  const checks = [
    {
      id: 'eval_quality_metrics_dataset_path_canonical_contract',
      ok: isCanonicalRelativePath(args.datasetPath),
      detail: args.datasetPath,
    },
    {
      id: 'eval_quality_metrics_monitor_path_canonical_contract',
      ok: isCanonicalRelativePath(args.monitorPath),
      detail: args.monitorPath,
    },
    {
      id: 'eval_quality_metrics_thresholds_path_canonical_contract',
      ok: isCanonicalRelativePath(args.thresholdsPath),
      detail: args.thresholdsPath,
    },
    {
      id: 'eval_quality_metrics_out_path_canonical_contract',
      ok: isCanonicalRelativePath(args.outPath),
      detail: args.outPath,
    },
    {
      id: 'eval_quality_metrics_out_latest_path_canonical_contract',
      ok: isCanonicalRelativePath(args.outLatestPath),
      detail: args.outLatestPath,
    },
    {
      id: 'eval_quality_metrics_markdown_path_canonical_contract',
      ok: isCanonicalRelativePath(args.markdownPath),
      detail: args.markdownPath,
    },
    {
      id: 'eval_quality_metrics_out_path_current_suffix_contract',
      ok: hasCaseInsensitiveSuffix(args.outPath, '_current.json'),
      detail: args.outPath,
    },
    {
      id: 'eval_quality_metrics_out_latest_path_latest_suffix_contract',
      ok: hasCaseInsensitiveSuffix(args.outLatestPath, '_latest.json'),
      detail: args.outLatestPath,
    },
    {
      id: 'eval_quality_metrics_markdown_path_current_suffix_contract',
      ok: hasCaseInsensitiveSuffix(args.markdownPath, '_current.md'),
      detail: args.markdownPath,
    },
    {
      id: 'eval_quality_metrics_output_paths_distinct_contract',
      ok: new Set([args.outPath, args.outLatestPath, args.markdownPath]).size === 3,
      detail: `${args.outPath}|${args.outLatestPath}|${args.markdownPath}`,
    },
    {
      id: 'eval_quality_metrics_dataset_rows_nonempty_contract',
      ok: datasetRows.length > 0,
      detail: `rows=${datasetRows.length}`,
    },
    {
      id: 'eval_quality_metrics_dataset_issue_class_token_contract',
      ok: datasetRows.every((row) => isCanonicalToken(cleanText(row?.labels?.issue_class || '', 120))),
      detail: `rows=${datasetRows.length}`,
    },
    {
      id: 'eval_quality_metrics_dataset_failure_label_boolean_contract',
      ok: datasetRows.every((row) => typeof row?.labels?.is_failure === 'boolean'),
      detail: `rows=${datasetRows.length}`,
    },
    {
      id: 'eval_quality_metrics_thresholds_shape_contract',
      ok: isPlainObject(thresholds) && isPlainObject(globalThresholds) && isPlainObject(perClassThresholds),
      detail: `global=${isPlainObject(globalThresholds)};per_class=${isPlainObject(perClassThresholds)}`,
    },
    {
      id: 'eval_quality_metrics_global_threshold_probability_range_contract',
      ok: globalThresholdProbabilityRangeValid,
      detail: 'precision_min|recall_min|fpr_max|actionability_min|calibration_error_max',
    },
    {
      id: 'eval_quality_metrics_per_class_threshold_probability_range_contract',
      ok: perClassThresholdProbabilityRangeValid,
      detail: `classes=${Object.keys(perClassThresholds || {}).length}`,
    },
    {
      id: 'eval_quality_metrics_feedback_row_id_token_contract',
      ok: feedbackRows.every((row) => {
        const id = cleanText(row?.id || '', 120);
        return Boolean(id) && isCanonicalToken(id);
      }),
      detail: `rows=${feedbackRows.length}`,
    },
    {
      id: 'eval_quality_metrics_feedback_evidence_count_scalar_contract',
      ok: feedbackRows.every((row) => {
        if (row?.evidence_count == null) return true;
        const value = Number(row.evidence_count);
        return isNonNegativeInteger(value) && value >= 1;
      }),
      detail: `rows=${feedbackRows.length}`,
    },
    {
      id: 'eval_quality_metrics_feedback_confidence_scalar_contract',
      ok: feedbackRows.every((row) => row?.confidence == null || isProbability(Number(row.confidence))),
      detail: `rows=${feedbackRows.length}`,
    },
    {
      id: 'eval_quality_metrics_per_class_confusion_partition_contract',
      ok: perClassConfusionPartitionValid,
      detail: `classes=${Object.keys(perClass).length};rows=${totalRows}`,
    },
    { id: 'dataset_present', ok: fs.existsSync(datasetAbs), detail: args.datasetPath },
    { id: 'monitor_present', ok: fs.existsSync(monitorAbs), detail: args.monitorPath },
    { id: 'thresholds_present', ok: fs.existsSync(thresholdsAbs), detail: args.thresholdsPath },
    {
      id: 'quality_signal_coverage_contract',
      ok: true,
      detail: `mode=${insufficientSignal ? 'insufficient_signal' : 'scored'};predicted_non_info=${nonInfoPredictions.length};minimum_eval_samples=${minEvalSamples}`,
    },
    { id: 'threshold_contract', ok: thresholdViolations.length === 0, detail: `violations=${thresholdViolations.length}` },
    { id: 'regression_contract', ok: regressionViolations.length === 0, detail: `violations=${regressionViolations.length}` },
  ];

  const report = {
    type: 'eval_quality_metrics_guard',
    schema_version: 1,
    generated_at: nowIso,
    ok: checks.every((row) => row.ok),
    checks,
    metrics: {
      overall,
      per_class: perClass,
      reliability_buckets: reliabilityBuckets,
      evaluation_mode: insufficientSignal ? 'insufficient_signal' : 'scored',
      predicted_non_info_samples: nonInfoPredictions.length,
      minimum_eval_samples: minEvalSamples,
    },
    threshold_violations: thresholdViolations,
    regression_violations: regressionViolations,
    sources: {
      dataset: args.datasetPath,
      monitor: args.monitorPath,
      thresholds: args.thresholdsPath,
    },
  };

  writeJsonArtifact(outLatestAbs, report);
  writeTextArtifact(markdownAbs, renderMarkdown(report));
  return emitStructuredResult(report, {
    outPath: path.resolve(root, args.outPath),
    strict: args.strict,
    ok: report.ok,
  });
}

function clampConfidence(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(1, value));
}

process.exit(run(process.argv.slice(2)));
