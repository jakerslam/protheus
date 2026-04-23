#!/usr/bin/env node
/* eslint-disable no-console */
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { appendJsonLine, emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/orchestration_gateway_fallback_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/ORCHESTRATION_GATEWAY_FALLBACK_GUARD_CURRENT.md';
const DEFAULT_POLICY_PATH = 'client/runtime/config/orchestration_quality_policy.json';
const TEST_NAMES = [
  'planning_execution::non_legacy_surface_fixture_fallback_rate_stays_below_threshold',
  'quality_surface::non_legacy_surface_fixture_quality_stays_within_surface_thresholds',
] as const;
const SURFACES = ['sdk', 'gateway', 'dashboard'] as const;

type ScriptArgs = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  policyPath: string;
};

type SurfaceMetricRow = {
  total?: number;
  fallback_rate?: number;
  low_confidence_rate?: number;
};

type SurfaceMetrics = {
  [key: string]: SurfaceMetricRow;
};

type AdapterFallbackPolicy = {
  min_surface_total?: Record<string, number>;
  max_fallback_rate?: Record<string, number>;
  max_low_confidence_rate?: Record<string, number>;
  required_surface_fields?: string[];
  min_surface_rows_present?: number;
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
  adapter_fallback?: AdapterFallbackPolicy;
};

const DEFAULT_REQUIRED_SURFACE_FIELDS = ['total', 'fallback_rate', 'low_confidence_rate'];

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

function parseSurfaceMetrics(output: string): SurfaceMetrics | null {
  const marker = output.match(/surface_quality_metrics=(\{.*\})/m);
  if (!marker) {
    return null;
  }
  try {
    return JSON.parse(marker[1]) as SurfaceMetrics;
  } catch {
    return null;
  }
}

function numberOrNull(value: unknown): number | null {
  return Number.isFinite(Number(value)) ? Number(value) : null;
}

function thresholdRows(policy: AdapterFallbackPolicy) {
  const minSurfaceTotal = policy.min_surface_total || {};
  const maxFallbackRate = policy.max_fallback_rate || {};
  const maxLowConfidenceRate = policy.max_low_confidence_rate || {};
  return { minSurfaceTotal, maxFallbackRate, maxLowConfidenceRate };
}

function requiredSurfaceFieldsFromPolicy(policy: AdapterFallbackPolicy): string[] {
  if (!Array.isArray(policy.required_surface_fields) || policy.required_surface_fields.length === 0) {
    return [...DEFAULT_REQUIRED_SURFACE_FIELDS];
  }
  return policy.required_surface_fields
    .map((field) => String(field || '').trim())
    .filter((field) => field.length > 0);
}

function collectMissingSurfaceFields(
  row: SurfaceMetricRow | null | undefined,
  requiredFields: string[],
): string[] {
  const metricRow = row || {};
  return requiredFields.filter((field) => numberOrNull((metricRow as Record<string, unknown>)[field]) == null);
}

function evaluateThresholds(
  metrics: SurfaceMetrics | null,
  policy: AdapterFallbackPolicy,
): string[] {
  const failures: string[] = [];
  if (!metrics) {
    failures.push('surface_metrics_missing');
    return failures;
  }
  const { minSurfaceTotal, maxFallbackRate, maxLowConfidenceRate } = thresholdRows(policy);
  const requiredSurfaceFields = requiredSurfaceFieldsFromPolicy(policy);
  const minSurfaceRowsPresent = numberOrNull(policy.min_surface_rows_present);
  const surfaceRowsPresent = SURFACES.filter((surface) => {
    const row = metrics[surface];
    return row != null && typeof row === 'object';
  }).length;
  if (minSurfaceRowsPresent != null && surfaceRowsPresent < minSurfaceRowsPresent) {
    failures.push(
      `surface_metric_rows_below_min:actual=${surfaceRowsPresent.toFixed(4)}:min=${minSurfaceRowsPresent.toFixed(4)}`,
    );
  }
  for (const surface of SURFACES) {
    const row = metrics[surface] || {};
    const missingFields = collectMissingSurfaceFields(row, requiredSurfaceFields);
    for (const field of missingFields) {
      failures.push(`missing_surface_metric_field:${surface}:${field}`);
    }
    const total = numberOrNull(row.total);
    const fallbackRate = numberOrNull(row.fallback_rate);
    const lowConfidenceRate = numberOrNull(row.low_confidence_rate);
    const minTotal = numberOrNull(minSurfaceTotal[surface]);
    const maxFallback = numberOrNull(maxFallbackRate[surface]);
    const maxLowConfidence = numberOrNull(maxLowConfidenceRate[surface]);
    if (total == null) {
      failures.push(`missing_total:${surface}`);
    } else if (minTotal != null && total < minTotal) {
      failures.push(
        `surface_total_below_min:${surface}:actual=${total.toFixed(4)}:min=${minTotal.toFixed(4)}`,
      );
    }
    if (fallbackRate == null) {
      failures.push(`missing_fallback_rate:${surface}`);
    } else if (maxFallback != null && fallbackRate > maxFallback) {
      failures.push(
        `fallback_rate_exceeded:${surface}:actual=${fallbackRate.toFixed(4)}:max=${maxFallback.toFixed(4)}`,
      );
    }
    if (lowConfidenceRate == null) {
      failures.push(`missing_low_confidence_rate:${surface}`);
    } else if (maxLowConfidence != null && lowConfidenceRate > maxLowConfidence) {
      failures.push(
        `low_confidence_rate_exceeded:${surface}:actual=${lowConfidenceRate.toFixed(4)}:max=${maxLowConfidence.toFixed(4)}`,
      );
    }
  }
  return failures;
}

function evaluateMetricConsistency(
  metrics: SurfaceMetrics | null,
  policy: AdapterFallbackPolicy,
): string[] {
  const failures: string[] = [];
  if (!metrics) return failures;
  const epsilon = Math.max(0, Number(policy.consistency_epsilon ?? 0));
  for (const surface of SURFACES) {
    const row = metrics[surface] || {};
    const total = numberOrNull(row.total);
    const fallbackRate = numberOrNull(row.fallback_rate);
    const lowConfidenceRate = numberOrNull(row.low_confidence_rate);
    if (total == null) {
      failures.push(`inconsistent_total_missing:${surface}`);
      continue;
    }
    if (total < 0) {
      failures.push(`inconsistent_total_negative:${surface}:value=${total.toFixed(4)}`);
    }
    if (!Number.isInteger(total)) {
      failures.push(`inconsistent_total_non_integral:${surface}:value=${total.toFixed(4)}`);
    }
    if (fallbackRate != null && (fallbackRate < 0 || fallbackRate > 1)) {
      failures.push(
        `inconsistent_fallback_rate_out_of_domain:${surface}:value=${fallbackRate.toFixed(4)}:expected=0..1`,
      );
    }
    if (lowConfidenceRate != null && (lowConfidenceRate < 0 || lowConfidenceRate > 1)) {
      failures.push(
        `inconsistent_low_confidence_rate_out_of_domain:${surface}:value=${lowConfidenceRate.toFixed(4)}:expected=0..1`,
      );
    }
    if (Math.abs(total) <= epsilon) {
      if (fallbackRate != null && Math.abs(fallbackRate) > epsilon) {
        failures.push(
          `inconsistent_nonzero_fallback_rate_with_zero_total:${surface}:value=${fallbackRate.toFixed(4)}`,
        );
      }
      if (lowConfidenceRate != null && Math.abs(lowConfidenceRate) > epsilon) {
        failures.push(
          `inconsistent_nonzero_low_confidence_rate_with_zero_total:${surface}:value=${lowConfidenceRate.toFixed(4)}`,
        );
      }
    }
    if (total > epsilon && fallbackRate == null) {
      failures.push(`inconsistent_missing_fallback_rate_with_positive_total:${surface}`);
    }
    if (total > epsilon && lowConfidenceRate == null) {
      failures.push(`inconsistent_missing_low_confidence_rate_with_positive_total:${surface}`);
    }
  }
  return failures;
}

function evaluateRatchet(
  metrics: SurfaceMetrics | null,
  policy: AdapterFallbackPolicy,
  previous: any,
): string[] {
  const failures: string[] = [];
  if (!metrics) return failures;
  const delta = Math.max(0, Number(policy.ratchet?.max_regression_delta || 0));
  const previousMetrics = previous?.surface_metrics;
  if (!previousMetrics || typeof previousMetrics !== 'object') return failures;
  for (const surface of SURFACES) {
    const currentFallback = numberOrNull(metrics[surface]?.fallback_rate);
    const currentLowConfidence = numberOrNull(metrics[surface]?.low_confidence_rate);
    const previousFallback = numberOrNull(previousMetrics?.[surface]?.fallback_rate);
    const previousLowConfidence = numberOrNull(previousMetrics?.[surface]?.low_confidence_rate);
    if (currentFallback != null && previousFallback != null && currentFallback > previousFallback + delta) {
      failures.push(
        `ratchet_fallback_regression:${surface}:current=${currentFallback.toFixed(4)}:previous=${previousFallback.toFixed(4)}:delta=${delta.toFixed(4)}`,
      );
    }
    if (
      currentLowConfidence != null
      && previousLowConfidence != null
      && currentLowConfidence > previousLowConfidence + delta
    ) {
      failures.push(
        `ratchet_low_confidence_regression:${surface}:current=${currentLowConfidence.toFixed(4)}:previous=${previousLowConfidence.toFixed(4)}:delta=${delta.toFixed(4)}`,
      );
    }
  }
  return failures;
}

function persistRatchet(policy: AdapterFallbackPolicy, snapshot: any): void {
  const latest = policy.paths?.latest || '';
  const history = policy.paths?.history || '';
  if (latest) writeJsonArtifact(latest, snapshot);
  if (history) appendJsonLine(history, snapshot);
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Orchestration Gateway Fallback Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push(`Policy: ${payload.inputs.policy_path}`);
  lines.push('');
  lines.push('## Commands');
  for (const row of payload.tests) {
    lines.push(`- ${row.command.join(' ')}`);
  }
  if (payload.surface_metrics) {
    lines.push('');
    lines.push('## Surface Metrics');
    lines.push('```json');
    lines.push(JSON.stringify(payload.surface_metrics, null, 2));
    lines.push('```');
  }
  if (Array.isArray(payload.surface_field_completeness) && payload.surface_field_completeness.length > 0) {
    lines.push('');
    lines.push('## Surface Metric Completeness');
    lines.push('| Surface | Required | Present | Missing |');
    lines.push('| --- | --- | --- | --- |');
    for (const row of payload.surface_field_completeness) {
      lines.push(
        `| ${row.surface} | ${row.required_count} | ${row.present_count} | ${row.missing_fields.join(',') || 'none'} |`,
      );
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
  const adapterPolicy = qualityPolicy.adapter_fallback || {};

  const runs = TEST_NAMES.map((name) => {
    const command = [
      'cargo',
      'test',
      '--manifest-path',
      'surface/orchestration/Cargo.toml',
      name,
      '--',
      '--exact',
      '--nocapture',
    ];
    const result = spawnSync(command[0], command.slice(1), {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    return {
      name,
      command,
      status: result.status ?? 1,
      signal: result.signal ?? null,
      stdout: String(result.stdout || ''),
      stderr: String(result.stderr || ''),
    };
  });

  const testsOk = runs.every((row) => row.status === 0);
  const combinedOutput = runs
    .map((row) => [row.stdout, row.stderr].filter(Boolean).join('\n').trim())
    .filter(Boolean)
    .join('\n\n');
  const surfaceMetrics = parseSurfaceMetrics(combinedOutput);
  const previousLatest = adapterPolicy.paths?.latest
    ? readJsonMaybe<any>(adapterPolicy.paths.latest)
    : null;
  const requiredSurfaceFields = requiredSurfaceFieldsFromPolicy(adapterPolicy);
  const surfaceFieldCompleteness = SURFACES.map((surface) => {
    const missingFields = collectMissingSurfaceFields(surfaceMetrics?.[surface], requiredSurfaceFields);
    return {
      surface,
      required_count: requiredSurfaceFields.length,
      present_count: requiredSurfaceFields.length - missingFields.length,
      missing_fields: missingFields,
    };
  });
  const missingSurfaceFieldCount = surfaceFieldCompleteness.reduce(
    (sum, row) => sum + row.missing_fields.length,
    0,
  );
  const surfaceRowsWithCompleteMetrics = surfaceFieldCompleteness.filter(
    (row) => row.missing_fields.length === 0,
  ).length;
  const policyFailures = evaluateThresholds(surfaceMetrics, adapterPolicy);
  const consistencyFailures = evaluateMetricConsistency(surfaceMetrics, adapterPolicy);
  const ratchetFailures = evaluateRatchet(surfaceMetrics, adapterPolicy, previousLatest);
  const surfaceMetricRowsPresent = SURFACES.filter((surface) => {
    const row = surfaceMetrics?.[surface];
    return row != null && typeof row === 'object';
  }).length;
  const ok = testsOk && policyFailures.length === 0 && consistencyFailures.length === 0 && ratchetFailures.length === 0;

  const payload = {
    ok,
    type: 'orchestration_gateway_fallback_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
      policy_path: args.policyPath,
    },
    test_names: TEST_NAMES,
    tests: runs.map((row) => ({
      test_name: row.name,
      command: row.command,
      exit_code: row.status,
      signal: row.signal,
    })),
    policy: adapterPolicy,
    surface_metrics: surfaceMetrics,
    policy_failures: policyFailures,
    consistency_failures: consistencyFailures,
    ratchet_failures: ratchetFailures,
    summary: {
      pass: ok,
      tests_pass: testsOk,
      failed_tests: runs.filter((row) => row.status !== 0).map((row) => row.name),
      policy_failure_count: policyFailures.length,
      consistency_failure_count: consistencyFailures.length,
      ratchet_failure_count: ratchetFailures.length,
      surface_metric_rows_present: surfaceMetricRowsPresent,
      required_surface_field_count: requiredSurfaceFields.length,
      missing_surface_field_count: missingSurfaceFieldCount,
      surface_rows_with_complete_metrics: surfaceRowsWithCompleteMetrics,
    },
    output_excerpt: combinedOutput,
    surface_field_completeness: surfaceFieldCompleteness,
  };

  if (ok && surfaceMetrics) {
    persistRatchet(adapterPolicy, {
      generated_at: payload.generated_at,
      revision: payload.revision,
      surface_metrics: surfaceMetrics,
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
