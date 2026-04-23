#!/usr/bin/env node
/* eslint-disable no-console */
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { appendJsonLine, emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/orchestration_planner_quality_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/ORCHESTRATION_PLANNER_QUALITY_GUARD_CURRENT.md';
const DEFAULT_POLICY_PATH = 'client/runtime/config/orchestration_quality_policy.json';
const TEST_NAME = 'quality_planner_runtime::planner_quality_fixture_metrics_stay_within_thresholds';

type ScriptArgs = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  policyPath: string;
};

type PlannerMetrics = {
  request_count?: number;
  average_candidate_count?: number;
  clarification_first_rate?: number;
  degraded_rate?: number;
  selected_plan_requires_clarification_rate?: number;
  selected_plan_degraded_rate?: number;
  heuristic_probe_rate?: number;
  zero_executable_candidate_rate?: number;
  all_candidates_require_clarification_rate?: number;
  all_candidates_degraded_rate?: number;
};

type PlannerPolicy = {
  min_request_count?: number;
  min_average_candidate_count?: number;
  max_clarification_first_rate?: number;
  max_degraded_rate?: number;
  max_selected_plan_requires_clarification_rate?: number;
  max_selected_plan_degraded_rate?: number;
  max_heuristic_probe_rate?: number;
  max_zero_executable_candidate_rate?: number;
  max_all_candidates_require_clarification_rate?: number;
  max_all_candidates_degraded_rate?: number;
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
  planner_quality?: PlannerPolicy;
};

const DEFAULT_REQUIRED_PLANNER_FIELDS: Array<keyof PlannerMetrics> = [
  'request_count',
  'average_candidate_count',
  'clarification_first_rate',
  'degraded_rate',
  'selected_plan_requires_clarification_rate',
  'selected_plan_degraded_rate',
  'heuristic_probe_rate',
  'zero_executable_candidate_rate',
  'all_candidates_require_clarification_rate',
  'all_candidates_degraded_rate',
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

function parsePlannerMetrics(output: string): PlannerMetrics | null {
  const marker = output.match(/planner_quality_metrics=(\{.*\})/m);
  if (!marker) {
    return null;
  }
  try {
    return JSON.parse(marker[1]) as PlannerMetrics;
  } catch {
    return null;
  }
}

function numberOrNull(value: unknown): number | null {
  return Number.isFinite(Number(value)) ? Number(value) : null;
}

function requiredPlannerFieldsFromPolicy(policy: PlannerPolicy): string[] {
  if (!Array.isArray(policy.required_metric_fields) || policy.required_metric_fields.length === 0) {
    return [...DEFAULT_REQUIRED_PLANNER_FIELDS];
  }
  return policy.required_metric_fields
    .map((field) => String(field || '').trim())
    .filter((field) => field.length > 0);
}

function collectMissingPlannerFields(metrics: PlannerMetrics | null, requiredFields: string[]): string[] {
  if (!metrics) return [...requiredFields];
  return requiredFields.filter((field) => numberOrNull((metrics as Record<string, unknown>)[field]) == null);
}

function evaluateThresholds(metrics: PlannerMetrics | null, policy: PlannerPolicy): string[] {
  const failures: string[] = [];
  if (!metrics) {
    failures.push('planner_metrics_missing');
    return failures;
  }
  const requestCount = numberOrNull(metrics.request_count);
  const averageCandidateCount = numberOrNull(metrics.average_candidate_count);
  const clarificationFirstRate = numberOrNull(metrics.clarification_first_rate);
  const degradedRate = numberOrNull(metrics.degraded_rate);
  const selectedPlanRequiresClarificationRate = numberOrNull(
    metrics.selected_plan_requires_clarification_rate,
  );
  const selectedPlanDegradedRate = numberOrNull(metrics.selected_plan_degraded_rate);
  const heuristicProbeRate = numberOrNull(metrics.heuristic_probe_rate);
  const zeroExecutableCandidateRate = numberOrNull(metrics.zero_executable_candidate_rate);
  const allCandidatesRequireClarificationRate = numberOrNull(
    metrics.all_candidates_require_clarification_rate,
  );
  const allCandidatesDegradedRate = numberOrNull(metrics.all_candidates_degraded_rate);

  const minAverageCandidateCount = numberOrNull(policy.min_average_candidate_count);
  const maxClarificationFirstRate = numberOrNull(policy.max_clarification_first_rate);
  const maxDegradedRate = numberOrNull(policy.max_degraded_rate);
  const maxSelectedPlanRequiresClarificationRate = numberOrNull(
    policy.max_selected_plan_requires_clarification_rate,
  );
  const maxSelectedPlanDegradedRate = numberOrNull(policy.max_selected_plan_degraded_rate);
  const maxHeuristicProbeRate = numberOrNull(policy.max_heuristic_probe_rate);
  const maxZeroExecutableCandidateRate = numberOrNull(policy.max_zero_executable_candidate_rate);
  const maxAllCandidatesRequireClarificationRate = numberOrNull(
    policy.max_all_candidates_require_clarification_rate,
  );
  const maxAllCandidatesDegradedRate = numberOrNull(policy.max_all_candidates_degraded_rate);
  const minRequestCount = numberOrNull(policy.min_request_count);
  const requiredMetricFields = requiredPlannerFieldsFromPolicy(policy);
  const missingMetricFields = collectMissingPlannerFields(metrics, requiredMetricFields);
  const maxMissingMetricFields = Math.max(0, Number(policy.max_missing_metric_fields ?? 0));

  if (missingMetricFields.length > maxMissingMetricFields) {
    failures.push(
      `missing_metric_fields_exceeded:actual=${missingMetricFields.length}:max=${maxMissingMetricFields}`,
    );
  }
  for (const field of missingMetricFields) {
    failures.push(`missing_metric_field:${field}`);
  }

  if (requestCount == null) {
    failures.push('missing_request_count');
  } else if (minRequestCount != null && requestCount < minRequestCount) {
    failures.push(
      `request_count_below_min:actual=${requestCount.toFixed(4)}:min=${minRequestCount.toFixed(4)}`,
    );
  }

  if (averageCandidateCount == null) {
    failures.push('missing_average_candidate_count');
  } else if (minAverageCandidateCount != null && averageCandidateCount < minAverageCandidateCount) {
    failures.push(
      `average_candidate_count_below_min:actual=${averageCandidateCount.toFixed(4)}:min=${minAverageCandidateCount.toFixed(4)}`,
    );
  }

  if (clarificationFirstRate == null) {
    failures.push('missing_clarification_first_rate');
  } else if (maxClarificationFirstRate != null && clarificationFirstRate > maxClarificationFirstRate) {
    failures.push(
      `clarification_first_rate_exceeded:actual=${clarificationFirstRate.toFixed(4)}:max=${maxClarificationFirstRate.toFixed(4)}`,
    );
  }

  if (degradedRate == null) {
    failures.push('missing_degraded_rate');
  } else if (maxDegradedRate != null && degradedRate > maxDegradedRate) {
    failures.push(`degraded_rate_exceeded:actual=${degradedRate.toFixed(4)}:max=${maxDegradedRate.toFixed(4)}`);
  }

  if (selectedPlanRequiresClarificationRate == null) {
    failures.push('missing_selected_plan_requires_clarification_rate');
  } else if (
    maxSelectedPlanRequiresClarificationRate != null
    && selectedPlanRequiresClarificationRate > maxSelectedPlanRequiresClarificationRate
  ) {
    failures.push(
      `selected_plan_requires_clarification_rate_exceeded:actual=${selectedPlanRequiresClarificationRate.toFixed(4)}:max=${maxSelectedPlanRequiresClarificationRate.toFixed(4)}`,
    );
  }

  if (selectedPlanDegradedRate == null) {
    failures.push('missing_selected_plan_degraded_rate');
  } else if (
    maxSelectedPlanDegradedRate != null
    && selectedPlanDegradedRate > maxSelectedPlanDegradedRate
  ) {
    failures.push(
      `selected_plan_degraded_rate_exceeded:actual=${selectedPlanDegradedRate.toFixed(4)}:max=${maxSelectedPlanDegradedRate.toFixed(4)}`,
    );
  }

  if (heuristicProbeRate == null) {
    failures.push('missing_heuristic_probe_rate');
  } else if (maxHeuristicProbeRate != null && heuristicProbeRate > maxHeuristicProbeRate) {
    failures.push(
      `heuristic_probe_rate_exceeded:actual=${heuristicProbeRate.toFixed(4)}:max=${maxHeuristicProbeRate.toFixed(4)}`,
    );
  }

  if (zeroExecutableCandidateRate == null) {
    failures.push('missing_zero_executable_candidate_rate');
  } else if (
    maxZeroExecutableCandidateRate != null
    && zeroExecutableCandidateRate > maxZeroExecutableCandidateRate
  ) {
    failures.push(
      `zero_executable_candidate_rate_exceeded:actual=${zeroExecutableCandidateRate.toFixed(4)}:max=${maxZeroExecutableCandidateRate.toFixed(4)}`,
    );
  }

  if (allCandidatesRequireClarificationRate == null) {
    failures.push('missing_all_candidates_require_clarification_rate');
  } else if (
    maxAllCandidatesRequireClarificationRate != null
    && allCandidatesRequireClarificationRate > maxAllCandidatesRequireClarificationRate
  ) {
    failures.push(
      `all_candidates_require_clarification_rate_exceeded:actual=${allCandidatesRequireClarificationRate.toFixed(4)}:max=${maxAllCandidatesRequireClarificationRate.toFixed(4)}`,
    );
  }

  if (allCandidatesDegradedRate == null) {
    failures.push('missing_all_candidates_degraded_rate');
  } else if (
    maxAllCandidatesDegradedRate != null
    && allCandidatesDegradedRate > maxAllCandidatesDegradedRate
  ) {
    failures.push(
      `all_candidates_degraded_rate_exceeded:actual=${allCandidatesDegradedRate.toFixed(4)}:max=${maxAllCandidatesDegradedRate.toFixed(4)}`,
    );
  }

  return failures;
}

function evaluateMetricConsistency(metrics: PlannerMetrics | null, policy: PlannerPolicy): string[] {
  const failures: string[] = [];
  if (!metrics) return failures;
  const epsilon = Math.max(0, Number(policy.consistency_epsilon ?? 0));

  const requestCount = numberOrNull(metrics.request_count);
  const averageCandidateCount = numberOrNull(metrics.average_candidate_count);
  const clarificationFirstRate = numberOrNull(metrics.clarification_first_rate);
  const degradedRate = numberOrNull(metrics.degraded_rate);
  const selectedPlanRequiresClarificationRate = numberOrNull(
    metrics.selected_plan_requires_clarification_rate,
  );
  const selectedPlanDegradedRate = numberOrNull(metrics.selected_plan_degraded_rate);
  const heuristicProbeRate = numberOrNull(metrics.heuristic_probe_rate);
  const zeroExecutableCandidateRate = numberOrNull(metrics.zero_executable_candidate_rate);
  const allCandidatesRequireClarificationRate = numberOrNull(
    metrics.all_candidates_require_clarification_rate,
  );
  const allCandidatesDegradedRate = numberOrNull(metrics.all_candidates_degraded_rate);

  const rateRows: Array<{ key: string; value: number | null }> = [
    { key: 'clarification_first_rate', value: clarificationFirstRate },
    { key: 'degraded_rate', value: degradedRate },
    {
      key: 'selected_plan_requires_clarification_rate',
      value: selectedPlanRequiresClarificationRate,
    },
    { key: 'selected_plan_degraded_rate', value: selectedPlanDegradedRate },
    { key: 'heuristic_probe_rate', value: heuristicProbeRate },
    { key: 'zero_executable_candidate_rate', value: zeroExecutableCandidateRate },
    {
      key: 'all_candidates_require_clarification_rate',
      value: allCandidatesRequireClarificationRate,
    },
    { key: 'all_candidates_degraded_rate', value: allCandidatesDegradedRate },
  ];

  for (const row of rateRows) {
    if (row.value == null) continue;
    if (row.value < 0 || row.value > 1) {
      failures.push(`inconsistent_${row.key}_out_of_domain:value=${row.value.toFixed(4)}:expected=0..1`);
    }
  }

  if (requestCount != null && requestCount < 0) {
    failures.push(`inconsistent_request_count_negative:value=${requestCount.toFixed(4)}`);
  }
  if (requestCount != null && !Number.isInteger(requestCount)) {
    failures.push(`inconsistent_request_count_non_integral:value=${requestCount.toFixed(4)}`);
  }
  if (averageCandidateCount != null && averageCandidateCount < 0) {
    failures.push(`inconsistent_average_candidate_count_negative:value=${averageCandidateCount.toFixed(4)}`);
  }
  if (
    requestCount != null
    && Math.abs(requestCount) <= epsilon
    && rateRows.some((row) => row.value != null && Math.abs(row.value) > epsilon)
  ) {
    failures.push('inconsistent_rates_with_zero_request_count');
  }
  if (
    requestCount != null
    && Math.abs(requestCount) <= epsilon
    && averageCandidateCount != null
    && averageCandidateCount > epsilon
  ) {
    failures.push(
      `inconsistent_average_candidate_count_with_zero_request_count:value=${averageCandidateCount.toFixed(4)}`,
    );
  }
  if (requestCount != null && requestCount > epsilon) {
    for (const row of rateRows) {
      if (row.value == null) {
        failures.push(`inconsistent_missing_${row.key}_with_positive_request_count`);
      }
    }
    if (averageCandidateCount == null) {
      failures.push('inconsistent_missing_average_candidate_count_with_positive_request_count');
    }
  }

  if (
    allCandidatesRequireClarificationRate != null
    && selectedPlanRequiresClarificationRate != null
    && allCandidatesRequireClarificationRate > selectedPlanRequiresClarificationRate
  ) {
    failures.push(
      `inconsistent_all_candidates_require_clarification_rate:all_candidates_require_clarification_rate=${allCandidatesRequireClarificationRate.toFixed(4)}:selected_plan_requires_clarification_rate=${selectedPlanRequiresClarificationRate.toFixed(4)}`,
    );
  } else if (
    allCandidatesRequireClarificationRate != null
    && selectedPlanRequiresClarificationRate == null
    && clarificationFirstRate != null
    && allCandidatesRequireClarificationRate > clarificationFirstRate
  ) {
    failures.push(
      `inconsistent_all_candidates_require_clarification_rate:all_candidates_require_clarification_rate=${allCandidatesRequireClarificationRate.toFixed(4)}:clarification_first_rate=${clarificationFirstRate.toFixed(4)}`,
    );
  }

  if (
    allCandidatesDegradedRate != null
    && selectedPlanDegradedRate != null
    && allCandidatesDegradedRate > selectedPlanDegradedRate
  ) {
    failures.push(
      `inconsistent_all_candidates_degraded_rate:all_candidates_degraded_rate=${allCandidatesDegradedRate.toFixed(4)}:selected_plan_degraded_rate=${selectedPlanDegradedRate.toFixed(4)}`,
    );
  } else if (
    allCandidatesDegradedRate != null
    && selectedPlanDegradedRate == null
    && degradedRate != null
    && allCandidatesDegradedRate > degradedRate
  ) {
    failures.push(
      `inconsistent_all_candidates_degraded_rate:all_candidates_degraded_rate=${allCandidatesDegradedRate.toFixed(4)}:degraded_rate=${degradedRate.toFixed(4)}`,
    );
  }
  if (
    allCandidatesRequireClarificationRate != null
    && zeroExecutableCandidateRate != null
    && allCandidatesRequireClarificationRate > zeroExecutableCandidateRate
  ) {
    failures.push(
      `inconsistent_all_candidates_require_clarification_rate_vs_zero_executable_candidate_rate:all_candidates_require_clarification_rate=${allCandidatesRequireClarificationRate.toFixed(4)}:zero_executable_candidate_rate=${zeroExecutableCandidateRate.toFixed(4)}`,
    );
  }
  if (
    selectedPlanRequiresClarificationRate != null
    && selectedPlanDegradedRate != null
    && selectedPlanRequiresClarificationRate + selectedPlanDegradedRate > 1 + epsilon
  ) {
    failures.push(
      `inconsistent_selected_plan_rate_sum_over_1:sum=${(selectedPlanRequiresClarificationRate + selectedPlanDegradedRate).toFixed(4)}`,
    );
  }
  if (
    allCandidatesRequireClarificationRate != null
    && allCandidatesDegradedRate != null
    && allCandidatesRequireClarificationRate + allCandidatesDegradedRate > 1 + epsilon
  ) {
    failures.push(
      `inconsistent_all_candidates_rate_sum_over_1:sum=${(allCandidatesRequireClarificationRate + allCandidatesDegradedRate).toFixed(4)}`,
    );
  }
  if (
    selectedPlanRequiresClarificationRate != null
    && clarificationFirstRate != null
    && selectedPlanRequiresClarificationRate > clarificationFirstRate + epsilon
  ) {
    failures.push(
      `inconsistent_selected_plan_requires_clarification_rate_vs_clarification_first_rate:selected_plan_requires_clarification_rate=${selectedPlanRequiresClarificationRate.toFixed(4)}:clarification_first_rate=${clarificationFirstRate.toFixed(4)}`,
    );
  }
  if (
    selectedPlanDegradedRate != null
    && degradedRate != null
    && selectedPlanDegradedRate > degradedRate + epsilon
  ) {
    failures.push(
      `inconsistent_selected_plan_degraded_rate_vs_degraded_rate:selected_plan_degraded_rate=${selectedPlanDegradedRate.toFixed(4)}:degraded_rate=${degradedRate.toFixed(4)}`,
    );
  }
  if (
    requestCount != null
    && requestCount > epsilon
    && averageCandidateCount != null
    && averageCandidateCount < 1
  ) {
    failures.push(
      `inconsistent_average_candidate_count_below_1_with_nonzero_requests:request_count=${requestCount.toFixed(4)}:average_candidate_count=${averageCandidateCount.toFixed(4)}`,
    );
  }
  return failures;
}

function evaluateRatchet(metrics: PlannerMetrics | null, policy: PlannerPolicy, previous: any): string[] {
  const failures: string[] = [];
  if (!metrics) return failures;
  const previousMetrics = previous?.metrics;
  if (!previousMetrics || typeof previousMetrics !== 'object') return failures;
  const delta = Math.max(0, Number(policy.ratchet?.max_regression_delta || 0));

  const currentRequestCount = numberOrNull(metrics.request_count);
  const previousRequestCount = numberOrNull(previousMetrics.request_count);
  if (
    currentRequestCount != null
    && previousRequestCount != null
    && currentRequestCount < previousRequestCount - delta
  ) {
    failures.push(
      `ratchet_request_count_regression:current=${currentRequestCount.toFixed(4)}:previous=${previousRequestCount.toFixed(4)}:delta=${delta.toFixed(4)}`,
    );
  }

  const currentAverage = numberOrNull(metrics.average_candidate_count);
  const previousAverage = numberOrNull(previousMetrics.average_candidate_count);
  if (currentAverage != null && previousAverage != null && currentAverage < previousAverage - delta) {
    failures.push(
      `ratchet_average_candidate_count_regression:current=${currentAverage.toFixed(4)}:previous=${previousAverage.toFixed(4)}:delta=${delta.toFixed(4)}`,
    );
  }

  const maxFields: Array<keyof PlannerMetrics> = [
    'clarification_first_rate',
    'degraded_rate',
    'heuristic_probe_rate',
    'zero_executable_candidate_rate',
    'all_candidates_require_clarification_rate',
    'all_candidates_degraded_rate',
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

function persistRatchet(policy: PlannerPolicy, snapshot: any): void {
  const latest = policy.paths?.latest || '';
  const history = policy.paths?.history || '';
  if (latest) writeJsonArtifact(latest, snapshot);
  if (history) appendJsonLine(history, snapshot);
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Orchestration Planner Quality Guard');
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
    lines.push('## Planner Metrics');
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
  const plannerPolicy = qualityPolicy.planner_quality || {};

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
  const metrics = parsePlannerMetrics(output);
  const previousLatest = plannerPolicy.paths?.latest
    ? readJsonMaybe<any>(plannerPolicy.paths.latest)
    : null;
  const requiredMetricFields = requiredPlannerFieldsFromPolicy(plannerPolicy);
  const missingMetricFields = collectMissingPlannerFields(metrics, requiredMetricFields);
  const policyFailures = evaluateThresholds(metrics, plannerPolicy);
  const consistencyFailures = evaluateMetricConsistency(metrics, plannerPolicy);
  const ratchetFailures = evaluateRatchet(metrics, plannerPolicy, previousLatest);
  const metricFieldsPresent = metrics
    ? Object.values(metrics).filter((value) => numberOrNull(value) != null).length
    : 0;
  const ok = testsOk && policyFailures.length === 0 && consistencyFailures.length === 0 && ratchetFailures.length === 0;

  const payload = {
    ok,
    type: 'orchestration_planner_quality_guard',
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
    policy: plannerPolicy,
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
    persistRatchet(plannerPolicy, {
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
