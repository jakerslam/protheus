#!/usr/bin/env node
/* eslint-disable no-console */
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { appendJsonLine, emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/orchestration_runtime_quality_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/ORCHESTRATION_RUNTIME_QUALITY_GUARD_CURRENT.md';
const DEFAULT_POLICY_PATH = 'client/runtime/config/orchestration_quality_policy.json';
const TEST_NAME = 'quality_planner_runtime::runtime_quality_telemetry_metrics_stay_within_thresholds';

type ScriptArgs = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  policyPath: string;
};

type RuntimeQualityMetrics = {
  sample_size_non_legacy?: number;
  fallback_rate_non_legacy?: number;
  heuristic_probe_rate_non_legacy?: number;
  clarification_rate_non_legacy?: number;
  zero_executable_rate_non_legacy?: number;
  all_candidates_degraded_rate_non_legacy?: number;
  average_candidate_count?: number;
};

type RuntimeQualityPolicy = {
  min_sample_size?: number;
  min_average_candidate_count?: number;
  max_non_legacy_fallback_rate?: number;
  max_non_legacy_heuristic_probe_rate?: number;
  max_non_legacy_clarification_rate?: number;
  max_non_legacy_zero_executable_rate?: number;
  max_non_legacy_all_candidates_degraded_rate?: number;
  max_non_legacy_fallback_plus_heuristic_rate?: number;
  max_non_legacy_clarification_plus_degraded_rate?: number;
  max_non_legacy_fallback_minus_zero_executable_rate?: number;
  required_metric_fields?: string[];
  max_missing_metric_fields?: number;
  consistency_epsilon?: number;
  ratchet?: {
    max_regression_delta?: number;
  };
  paths?: {
    latest?: string;
    history?: string;
  };
};

type OrchestrationQualityPolicy = {
  runtime_quality?: RuntimeQualityPolicy;
};

const DEFAULT_REQUIRED_RUNTIME_FIELDS: Array<keyof RuntimeQualityMetrics> = [
  'sample_size_non_legacy',
  'fallback_rate_non_legacy',
  'heuristic_probe_rate_non_legacy',
  'clarification_rate_non_legacy',
  'zero_executable_rate_non_legacy',
  'all_candidates_degraded_rate_non_legacy',
  'average_candidate_count',
];

function resolveArgs(argv: string[]): ScriptArgs {
  return {
    strict: argv.includes('--strict') || parseBool(readFlag(argv, 'strict'), false),
    outJson: readFlag(argv, 'out-json') || DEFAULT_OUT_JSON,
    outMarkdown: readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD,
    policyPath: readFlag(argv, 'policy') || DEFAULT_POLICY_PATH,
  };
}

function readJsonMaybe<T>(filePath: string): T | null {
  try {
    return JSON.parse(fs.readFileSync(path.resolve(ROOT, filePath), 'utf8')) as T;
  } catch {
    return null;
  }
}

function parseRuntimeMetrics(output: string): RuntimeQualityMetrics | null {
  const marker = output.match(/runtime_quality_metrics=(\{.*\})/m);
  if (!marker) {
    return null;
  }
  try {
    return JSON.parse(marker[1]) as RuntimeQualityMetrics;
  } catch {
    return null;
  }
}

function numberOrNull(value: unknown): number | null {
  return Number.isFinite(Number(value)) ? Number(value) : null;
}

function requiredMetricFieldsFromPolicy(policy: RuntimeQualityPolicy): string[] {
  if (!Array.isArray(policy.required_metric_fields) || policy.required_metric_fields.length === 0) {
    return [...DEFAULT_REQUIRED_RUNTIME_FIELDS];
  }
  return policy.required_metric_fields
    .map((field) => String(field || '').trim())
    .filter((field) => field.length > 0);
}

function collectMissingMetricFields(metrics: RuntimeQualityMetrics | null, requiredFields: string[]): string[] {
  if (!metrics) return [...requiredFields];
  return requiredFields.filter((field) => numberOrNull((metrics as Record<string, unknown>)[field]) == null);
}

function evaluateThresholds(metrics: RuntimeQualityMetrics | null, policy: RuntimeQualityPolicy): string[] {
  const failures: string[] = [];
  if (!metrics) {
    failures.push('runtime_quality_metrics_missing');
    return failures;
  }

  const sampleSize = numberOrNull(metrics.sample_size_non_legacy);
  const averageCandidateCount = numberOrNull(metrics.average_candidate_count);
  const fallbackRate = numberOrNull(metrics.fallback_rate_non_legacy);
  const heuristicProbeRate = numberOrNull(metrics.heuristic_probe_rate_non_legacy);
  const clarificationRate = numberOrNull(metrics.clarification_rate_non_legacy);
  const zeroExecutableRate = numberOrNull(metrics.zero_executable_rate_non_legacy);
  const allCandidatesDegradedRate = numberOrNull(metrics.all_candidates_degraded_rate_non_legacy);

  const minSampleSize = numberOrNull(policy.min_sample_size);
  const minAverageCandidateCount = numberOrNull(policy.min_average_candidate_count);
  const maxFallbackRate = numberOrNull(policy.max_non_legacy_fallback_rate);
  const maxHeuristicProbeRate = numberOrNull(policy.max_non_legacy_heuristic_probe_rate);
  const maxClarificationRate = numberOrNull(policy.max_non_legacy_clarification_rate);
  const maxZeroExecutableRate = numberOrNull(policy.max_non_legacy_zero_executable_rate);
  const maxAllCandidatesDegradedRate = numberOrNull(
    policy.max_non_legacy_all_candidates_degraded_rate,
  );
  const maxFallbackPlusHeuristicRate = numberOrNull(
    policy.max_non_legacy_fallback_plus_heuristic_rate,
  );
  const maxClarificationPlusDegradedRate = numberOrNull(
    policy.max_non_legacy_clarification_plus_degraded_rate,
  );
  const maxFallbackMinusZeroExecutableRate = numberOrNull(
    policy.max_non_legacy_fallback_minus_zero_executable_rate,
  );
  const requiredMetricFields = requiredMetricFieldsFromPolicy(policy);
  const missingMetricFields = collectMissingMetricFields(metrics, requiredMetricFields);
  const maxMissingMetricFields = Math.max(0, Number(policy.max_missing_metric_fields ?? 0));
  if (missingMetricFields.length > maxMissingMetricFields) {
    failures.push(
      `missing_metric_fields_exceeded:actual=${missingMetricFields.length}:max=${maxMissingMetricFields}`,
    );
  }
  for (const field of missingMetricFields) {
    failures.push(`missing_metric_field:${field}`);
  }

  if (sampleSize == null) {
    failures.push('missing_sample_size_non_legacy');
  } else if (minSampleSize != null && sampleSize < minSampleSize) {
    failures.push(`sample_size_below_min:actual=${sampleSize.toFixed(4)}:min=${minSampleSize.toFixed(4)}`);
  }

  if (averageCandidateCount == null) {
    failures.push('missing_average_candidate_count');
  } else if (minAverageCandidateCount != null && averageCandidateCount < minAverageCandidateCount) {
    failures.push(
      `average_candidate_count_below_min:actual=${averageCandidateCount.toFixed(4)}:min=${minAverageCandidateCount.toFixed(4)}`,
    );
  }

  if (fallbackRate == null) {
    failures.push('missing_fallback_rate_non_legacy');
  } else if (maxFallbackRate != null && fallbackRate > maxFallbackRate) {
    failures.push(
      `fallback_rate_non_legacy_exceeded:actual=${fallbackRate.toFixed(4)}:max=${maxFallbackRate.toFixed(4)}`,
    );
  }

  if (heuristicProbeRate == null) {
    failures.push('missing_heuristic_probe_rate_non_legacy');
  } else if (maxHeuristicProbeRate != null && heuristicProbeRate > maxHeuristicProbeRate) {
    failures.push(
      `heuristic_probe_rate_non_legacy_exceeded:actual=${heuristicProbeRate.toFixed(4)}:max=${maxHeuristicProbeRate.toFixed(4)}`,
    );
  }

  if (clarificationRate == null) {
    failures.push('missing_clarification_rate_non_legacy');
  } else if (maxClarificationRate != null && clarificationRate > maxClarificationRate) {
    failures.push(
      `clarification_rate_non_legacy_exceeded:actual=${clarificationRate.toFixed(4)}:max=${maxClarificationRate.toFixed(4)}`,
    );
  }

  if (zeroExecutableRate == null) {
    failures.push('missing_zero_executable_rate_non_legacy');
  } else if (maxZeroExecutableRate != null && zeroExecutableRate > maxZeroExecutableRate) {
    failures.push(
      `zero_executable_rate_non_legacy_exceeded:actual=${zeroExecutableRate.toFixed(4)}:max=${maxZeroExecutableRate.toFixed(4)}`,
    );
  }

  if (allCandidatesDegradedRate == null) {
    failures.push('missing_all_candidates_degraded_rate_non_legacy');
  } else if (
    maxAllCandidatesDegradedRate != null
    && allCandidatesDegradedRate > maxAllCandidatesDegradedRate
  ) {
    failures.push(
      `all_candidates_degraded_rate_non_legacy_exceeded:actual=${allCandidatesDegradedRate.toFixed(4)}:max=${maxAllCandidatesDegradedRate.toFixed(4)}`,
    );
  }

  if (
    fallbackRate != null
    && heuristicProbeRate != null
    && maxFallbackPlusHeuristicRate != null
    && fallbackRate + heuristicProbeRate > maxFallbackPlusHeuristicRate
  ) {
    failures.push(
      `fallback_plus_heuristic_rate_non_legacy_exceeded:actual=${(fallbackRate + heuristicProbeRate).toFixed(4)}:max=${maxFallbackPlusHeuristicRate.toFixed(4)}`,
    );
  }

  if (
    clarificationRate != null
    && allCandidatesDegradedRate != null
    && maxClarificationPlusDegradedRate != null
    && clarificationRate + allCandidatesDegradedRate > maxClarificationPlusDegradedRate
  ) {
    failures.push(
      `clarification_plus_all_candidates_degraded_rate_non_legacy_exceeded:actual=${(clarificationRate + allCandidatesDegradedRate).toFixed(4)}:max=${maxClarificationPlusDegradedRate.toFixed(4)}`,
    );
  }

  if (
    fallbackRate != null
    && zeroExecutableRate != null
    && maxFallbackMinusZeroExecutableRate != null
    && fallbackRate - zeroExecutableRate > maxFallbackMinusZeroExecutableRate
  ) {
    failures.push(
      `fallback_minus_zero_executable_rate_non_legacy_exceeded:actual=${(fallbackRate - zeroExecutableRate).toFixed(4)}:max=${maxFallbackMinusZeroExecutableRate.toFixed(4)}`,
    );
  }

  return failures;
}

function evaluateMetricConsistency(
  metrics: RuntimeQualityMetrics | null,
  policy: RuntimeQualityPolicy,
): string[] {
  const failures: string[] = [];
  if (!metrics) return failures;
  const epsilon = Math.max(0, Number(policy.consistency_epsilon ?? 0));

  const sampleSize = numberOrNull(metrics.sample_size_non_legacy);
  const averageCandidateCount = numberOrNull(metrics.average_candidate_count);
  const fallbackRate = numberOrNull(metrics.fallback_rate_non_legacy);
  const heuristicProbeRate = numberOrNull(metrics.heuristic_probe_rate_non_legacy);
  const clarificationRate = numberOrNull(metrics.clarification_rate_non_legacy);
  const zeroExecutableRate = numberOrNull(metrics.zero_executable_rate_non_legacy);
  const allCandidatesDegradedRate = numberOrNull(metrics.all_candidates_degraded_rate_non_legacy);

  const rateRows: Array<{ key: string; value: number | null }> = [
    { key: 'fallback_rate_non_legacy', value: fallbackRate },
    { key: 'heuristic_probe_rate_non_legacy', value: heuristicProbeRate },
    { key: 'clarification_rate_non_legacy', value: clarificationRate },
    { key: 'zero_executable_rate_non_legacy', value: zeroExecutableRate },
    { key: 'all_candidates_degraded_rate_non_legacy', value: allCandidatesDegradedRate },
  ];

  for (const row of rateRows) {
    if (row.value == null) continue;
    if (row.value < 0 || row.value > 1) {
      failures.push(
        `inconsistent_${row.key}_out_of_domain:value=${row.value.toFixed(4)}:expected=0..1`,
      );
    }
  }

  if (sampleSize != null && sampleSize < 0) {
    failures.push(`inconsistent_sample_size_non_legacy_negative:value=${sampleSize.toFixed(4)}`);
  }
  if (sampleSize != null && !Number.isInteger(sampleSize)) {
    failures.push(`inconsistent_sample_size_non_legacy_non_integral:value=${sampleSize.toFixed(4)}`);
  }
  if (averageCandidateCount != null && averageCandidateCount < 0) {
    failures.push(`inconsistent_average_candidate_count_negative:value=${averageCandidateCount.toFixed(4)}`);
  }

  if (
    sampleSize != null
    && Math.abs(sampleSize) <= epsilon
    && rateRows.some((row) => row.value != null && Math.abs(row.value) > epsilon)
  ) {
    failures.push('inconsistent_non_legacy_rates_with_zero_sample_size');
  }
  if (
    sampleSize != null
    && Math.abs(sampleSize) <= epsilon
    && averageCandidateCount != null
    && averageCandidateCount > epsilon
  ) {
    failures.push(
      `inconsistent_average_candidate_count_with_zero_sample_size:value=${averageCandidateCount.toFixed(4)}`,
    );
  }
  if (sampleSize != null && sampleSize > epsilon) {
    for (const row of rateRows) {
      if (row.value == null) {
        failures.push(`inconsistent_missing_${row.key}_with_positive_sample_size`);
      }
    }
    if (averageCandidateCount == null) {
      failures.push('inconsistent_missing_average_candidate_count_with_positive_sample_size');
    }
  }
  if (
    allCandidatesDegradedRate != null
    && zeroExecutableRate != null
    && allCandidatesDegradedRate > zeroExecutableRate + epsilon
  ) {
    failures.push(
      `inconsistent_all_candidates_degraded_rate_vs_zero_executable_rate_non_legacy:all_candidates_degraded_rate_non_legacy=${allCandidatesDegradedRate.toFixed(4)}:zero_executable_rate_non_legacy=${zeroExecutableRate.toFixed(4)}`,
    );
  }
  if (
    clarificationRate != null
    && zeroExecutableRate != null
    && clarificationRate > zeroExecutableRate + epsilon
  ) {
    failures.push(
      `inconsistent_clarification_rate_vs_zero_executable_rate_non_legacy:clarification_rate_non_legacy=${clarificationRate.toFixed(4)}:zero_executable_rate_non_legacy=${zeroExecutableRate.toFixed(4)}`,
    );
  }
  if (
    allCandidatesDegradedRate != null
    && clarificationRate != null
    && allCandidatesDegradedRate > clarificationRate + epsilon
  ) {
    failures.push(
      `inconsistent_all_candidates_degraded_rate_vs_clarification_rate_non_legacy:all_candidates_degraded_rate_non_legacy=${allCandidatesDegradedRate.toFixed(4)}:clarification_rate_non_legacy=${clarificationRate.toFixed(4)}`,
    );
  }
  if (
    fallbackRate != null
    && zeroExecutableRate != null
    && fallbackRate > zeroExecutableRate + epsilon
  ) {
    failures.push(
      `inconsistent_fallback_rate_vs_zero_executable_rate_non_legacy:fallback_rate_non_legacy=${fallbackRate.toFixed(4)}:zero_executable_rate_non_legacy=${zeroExecutableRate.toFixed(4)}`,
    );
  }
  if (
    fallbackRate != null
    && heuristicProbeRate != null
    && fallbackRate + heuristicProbeRate > 1 + epsilon
  ) {
    failures.push(
      `inconsistent_fallback_plus_heuristic_rate_non_legacy_over_1:sum=${(fallbackRate + heuristicProbeRate).toFixed(4)}`,
    );
  }
  if (
    clarificationRate != null
    && allCandidatesDegradedRate != null
    && clarificationRate + allCandidatesDegradedRate > 1 + epsilon
  ) {
    failures.push(
      `inconsistent_clarification_plus_all_candidates_degraded_rate_non_legacy_over_1:sum=${(clarificationRate + allCandidatesDegradedRate).toFixed(4)}`,
    );
  }

  return failures;
}

function evaluateRatchet(
  metrics: RuntimeQualityMetrics | null,
  policy: RuntimeQualityPolicy,
  previous: any,
): string[] {
  const failures: string[] = [];
  if (!metrics) return failures;
  const previousMetrics = previous?.metrics;
  if (!previousMetrics || typeof previousMetrics !== 'object') return failures;
  const delta = Math.max(0, Number(policy.ratchet?.max_regression_delta || 0));

  const currentSampleSize = numberOrNull(metrics.sample_size_non_legacy);
  const previousSampleSize = numberOrNull(previousMetrics.sample_size_non_legacy);
  if (currentSampleSize != null && previousSampleSize != null && currentSampleSize < previousSampleSize - delta) {
    failures.push(
      `ratchet_sample_size_non_legacy_regression:current=${currentSampleSize.toFixed(4)}:previous=${previousSampleSize.toFixed(4)}:delta=${delta.toFixed(4)}`,
    );
  }

  const currentAverage = numberOrNull(metrics.average_candidate_count);
  const previousAverage = numberOrNull(previousMetrics.average_candidate_count);
  if (currentAverage != null && previousAverage != null && currentAverage < previousAverage - delta) {
    failures.push(
      `ratchet_average_candidate_count_regression:current=${currentAverage.toFixed(4)}:previous=${previousAverage.toFixed(4)}:delta=${delta.toFixed(4)}`,
    );
  }

  const maxFields: Array<keyof RuntimeQualityMetrics> = [
    'fallback_rate_non_legacy',
    'heuristic_probe_rate_non_legacy',
    'clarification_rate_non_legacy',
    'zero_executable_rate_non_legacy',
    'all_candidates_degraded_rate_non_legacy',
  ];
  for (const field of maxFields) {
    const current = numberOrNull(metrics[field]);
    const prior = numberOrNull(previousMetrics[field]);
    if (current != null && prior != null && current > prior + delta) {
      failures.push(
        `ratchet_${field}_regression:current=${current.toFixed(4)}:previous=${prior.toFixed(4)}:delta=${delta.toFixed(4)}`,
      );
    }
  }

  return failures;
}

function persistRatchet(policy: RuntimeQualityPolicy, snapshot: any): void {
  const latest = policy.paths?.latest || '';
  const history = policy.paths?.history || '';
  if (latest) writeJsonArtifact(latest, snapshot);
  if (history) appendJsonLine(history, snapshot);
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Orchestration Runtime Quality Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push(`Policy: ${payload.inputs.policy_path}`);
  lines.push('');
  lines.push('## Command');
  lines.push(`- ${payload.command.join(' ')}`);
  if (payload.metrics) {
    lines.push('');
    lines.push('## Runtime Quality Metrics');
    lines.push('```json');
    lines.push(JSON.stringify(payload.metrics, null, 2));
    lines.push('```');
  }
  if (payload.metric_completeness) {
    lines.push('');
    lines.push('## Metric Completeness');
    lines.push(`- required: ${payload.metric_completeness.required_metric_fields.length}`);
    lines.push(`- present: ${payload.metric_completeness.present_metric_fields}`);
    lines.push(`- missing: ${payload.metric_completeness.missing_metric_fields.length}`);
    if (payload.metric_completeness.missing_metric_fields.length > 0) {
      for (const row of payload.metric_completeness.missing_metric_fields) lines.push(`- missing_field: ${row}`);
    }
  }
  if (Array.isArray(payload.policy_failures) && payload.policy_failures.length > 0) {
    lines.push('');
    lines.push('## Policy Failures');
    for (const row of payload.policy_failures) lines.push(`- ${row}`);
  }
  if (Array.isArray(payload.consistency_failures) && payload.consistency_failures.length > 0) {
    lines.push('');
    lines.push('## Consistency Failures');
    for (const row of payload.consistency_failures) lines.push(`- ${row}`);
  }
  if (Array.isArray(payload.ratchet_failures) && payload.ratchet_failures.length > 0) {
    lines.push('');
    lines.push('## Ratchet Failures');
    for (const row of payload.ratchet_failures) lines.push(`- ${row}`);
  }
  lines.push('');
  lines.push('## Output');
  lines.push('```text');
  lines.push(String(payload.output_excerpt || '').trim().slice(0, 6000));
  lines.push('```');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[]): number {
  const args = resolveArgs(argv);
  const qualityPolicy = readJsonMaybe<OrchestrationQualityPolicy>(args.policyPath) || {};
  const runtimePolicy = qualityPolicy.runtime_quality || {};

  const command = [
    'cargo',
    'test',
    '--manifest-path',
    'surface/orchestration/Cargo.toml',
    TEST_NAME,
    '--',
    '--exact',
    '--nocapture',
  ];
  const result = spawnSync(command[0], command.slice(1), {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  const testsOk = result.status === 0;
  const output = [String(result.stdout || ''), String(result.stderr || '')]
    .filter(Boolean)
    .join('\n')
    .trim();
  const metrics = parseRuntimeMetrics(output);
  const previousLatest = runtimePolicy.paths?.latest
    ? readJsonMaybe<any>(runtimePolicy.paths.latest)
    : null;
  const requiredMetricFields = requiredMetricFieldsFromPolicy(runtimePolicy);
  const missingMetricFields = collectMissingMetricFields(metrics, requiredMetricFields);
  const policyFailures = evaluateThresholds(metrics, runtimePolicy);
  const consistencyFailures = evaluateMetricConsistency(metrics, runtimePolicy);
  const ratchetFailures = evaluateRatchet(metrics, runtimePolicy, previousLatest);
  const metricFieldsPresent = metrics
    ? Object.values(metrics).filter((value) => numberOrNull(value) != null).length
    : 0;
  const ok =
    testsOk
    && policyFailures.length === 0
    && consistencyFailures.length === 0
    && ratchetFailures.length === 0;

  const payload = {
    ok,
    type: 'orchestration_runtime_quality_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
      policy_path: args.policyPath,
    },
    test_name: TEST_NAME,
    command,
    policy: runtimePolicy,
    summary: {
      pass: ok,
      tests_pass: testsOk,
      exit_code: result.status ?? 1,
      signal: result.signal ?? null,
      policy_failure_count: policyFailures.length,
      consistency_failure_count: consistencyFailures.length,
      ratchet_failure_count: ratchetFailures.length,
      metric_fields_present: metricFieldsPresent,
      required_metric_field_count: requiredMetricFields.length,
      missing_metric_field_count: missingMetricFields.length,
    },
    metrics,
    metric_completeness: {
      required_metric_fields: requiredMetricFields,
      missing_metric_fields: missingMetricFields,
      present_metric_fields: requiredMetricFields.length - missingMetricFields.length,
    },
    policy_failures: policyFailures,
    consistency_failures: consistencyFailures,
    ratchet_failures: ratchetFailures,
    output_excerpt: output,
  };

  if (ok && metrics) {
    persistRatchet(runtimePolicy, {
      generated_at: payload.generated_at,
      revision: payload.revision,
      metrics,
      policy_path: args.policyPath,
    });
  }

  writeTextArtifact(args.outMarkdown, toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok,
  });
}

process.exit(run(process.argv.slice(2)));
