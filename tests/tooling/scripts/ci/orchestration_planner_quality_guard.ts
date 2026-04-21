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
const TEST_NAME = 'planner_quality_fixture_metrics_stay_within_thresholds';

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
  max_heuristic_probe_rate?: number;
  max_zero_executable_candidate_rate?: number;
  max_all_candidates_require_clarification_rate?: number;
  max_all_candidates_degraded_rate?: number;
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
  const heuristicProbeRate = numberOrNull(metrics.heuristic_probe_rate);
  const zeroExecutableCandidateRate = numberOrNull(metrics.zero_executable_candidate_rate);
  const allCandidatesRequireClarificationRate = numberOrNull(
    metrics.all_candidates_require_clarification_rate,
  );
  const allCandidatesDegradedRate = numberOrNull(metrics.all_candidates_degraded_rate);

  const minAverageCandidateCount = numberOrNull(policy.min_average_candidate_count);
  const maxClarificationFirstRate = numberOrNull(policy.max_clarification_first_rate);
  const maxDegradedRate = numberOrNull(policy.max_degraded_rate);
  const maxHeuristicProbeRate = numberOrNull(policy.max_heuristic_probe_rate);
  const maxZeroExecutableCandidateRate = numberOrNull(policy.max_zero_executable_candidate_rate);
  const maxAllCandidatesRequireClarificationRate = numberOrNull(
    policy.max_all_candidates_require_clarification_rate,
  );
  const maxAllCandidatesDegradedRate = numberOrNull(policy.max_all_candidates_degraded_rate);
  const minRequestCount = numberOrNull(policy.min_request_count);

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

function evaluateMetricConsistency(metrics: PlannerMetrics | null): string[] {
  const failures: string[] = [];
  if (!metrics) return failures;

  const clarificationFirstRate = numberOrNull(metrics.clarification_first_rate);
  const degradedRate = numberOrNull(metrics.degraded_rate);
  const zeroExecutableCandidateRate = numberOrNull(metrics.zero_executable_candidate_rate);
  const allCandidatesRequireClarificationRate = numberOrNull(
    metrics.all_candidates_require_clarification_rate,
  );
  const allCandidatesDegradedRate = numberOrNull(metrics.all_candidates_degraded_rate);

  if (
    clarificationFirstRate != null
    && allCandidatesRequireClarificationRate != null
    && allCandidatesRequireClarificationRate > clarificationFirstRate
  ) {
    failures.push(
      `inconsistent_all_candidates_require_clarification_rate:all_candidates_require_clarification_rate=${allCandidatesRequireClarificationRate.toFixed(4)}:clarification_first_rate=${clarificationFirstRate.toFixed(4)}`,
    );
  }

  if (degradedRate != null && allCandidatesDegradedRate != null && allCandidatesDegradedRate > degradedRate) {
    failures.push(
      `inconsistent_all_candidates_degraded_rate:all_candidates_degraded_rate=${allCandidatesDegradedRate.toFixed(4)}:degraded_rate=${degradedRate.toFixed(4)}`,
    );
  }

  if (zeroExecutableCandidateRate != null && clarificationFirstRate != null && degradedRate != null) {
    const upperBound = Math.min(1, clarificationFirstRate + degradedRate);
    if (zeroExecutableCandidateRate > upperBound) {
      failures.push(
        `inconsistent_zero_executable_candidate_rate_upper_bound:zero_executable_candidate_rate=${zeroExecutableCandidateRate.toFixed(4)}:upper_bound=${upperBound.toFixed(4)}`,
      );
    }
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
  const policyFailures = evaluateThresholds(metrics, plannerPolicy);
  const consistencyFailures = evaluateMetricConsistency(metrics);
  const ratchetFailures = evaluateRatchet(metrics, plannerPolicy, previousLatest);
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
    },
    metrics,
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
