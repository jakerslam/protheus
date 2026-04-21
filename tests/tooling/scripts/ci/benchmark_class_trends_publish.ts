#!/usr/bin/env node
/* eslint-disable no-console */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

const ROOT = resolve(__dirname, '..', '..', '..', '..');
const DEFAULT_CURRENT_REPORT = 'docs/client/reports/benchmark_matrix_run_latest.json';
const DEFAULT_BASELINE_REPORT = 'docs/client/reports/benchmark_matrix_run_2026-03-06.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/benchmark_class_trends_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/BENCHMARK_CLASS_TRENDS_CURRENT.md';

type Direction = 'higher_is_better' | 'lower_is_better';

type ClassMetric = {
  id: string;
  label: string;
  metric: string;
  direction: Direction;
  current: number | null;
  baseline: number | null;
  raw_delta_pct: number | null;
  improvement_pct: number | null;
  trend: 'improved' | 'regressed' | 'flat' | 'unknown';
  current_source: string;
  baseline_source: string;
};

type Payload = {
  ok: boolean;
  type: string;
  strict: boolean;
  generated_at: string;
  current_report_path: string;
  baseline_report_path: string;
  missing_metrics: string[];
  class_metrics: ClassMetric[];
};

function parseBool(raw: string, fallback: boolean): boolean {
  const normalized = String(raw || '').trim().toLowerCase();
  if (!normalized) return fallback;
  if (['1', 'true', 'yes', 'on'].includes(normalized)) return true;
  if (['0', 'false', 'no', 'off'].includes(normalized)) return false;
  return fallback;
}

function parseArgs(argv: string[]) {
  const out = {
    strict: false,
    currentReport: DEFAULT_CURRENT_REPORT,
    baselineReport: DEFAULT_BASELINE_REPORT,
    outJson: DEFAULT_OUT_JSON,
    outMarkdown: DEFAULT_OUT_MARKDOWN,
  };
  for (const rawToken of argv) {
    const token = String(rawToken || '').trim();
    if (!token) continue;
    if (token === '--strict' || token === '--strict=1') {
      out.strict = true;
      continue;
    }
    if (token.startsWith('--strict=')) {
      out.strict = parseBool(token.slice('--strict='.length), out.strict);
      continue;
    }
    if (token.startsWith('--current=')) {
      out.currentReport = token.slice('--current='.length).trim() || out.currentReport;
      continue;
    }
    if (token.startsWith('--baseline=')) {
      out.baselineReport = token.slice('--baseline='.length).trim() || out.baselineReport;
      continue;
    }
    if (token.startsWith('--out-json=')) {
      out.outJson = token.slice('--out-json='.length).trim() || out.outJson;
      continue;
    }
    if (token.startsWith('--out-markdown=')) {
      out.outMarkdown = token.slice('--out-markdown='.length).trim() || out.outMarkdown;
    }
  }
  return out;
}

function ensureParent(filePath: string): void {
  mkdirSync(dirname(filePath), { recursive: true });
}

function readJson(absPath: string): any | null {
  try {
    return JSON.parse(readFileSync(absPath, 'utf8'));
  } catch {
    return null;
  }
}

function asFinite(value: unknown): number {
  const num = Number(value);
  return Number.isFinite(num) ? num : Number.NaN;
}

function firstFinite(values: unknown[]): number {
  for (const value of values) {
    const num = asFinite(value);
    if (Number.isFinite(num)) return num;
  }
  return Number.NaN;
}

function findNumericByKey(input: any, key: string, depth = 0): number {
  if (!input || typeof input !== 'object' || depth > 8) return Number.NaN;
  if (Object.prototype.hasOwnProperty.call(input, key)) {
    const value = asFinite((input as Record<string, unknown>)[key]);
    if (Number.isFinite(value)) return value;
  }
  if (Array.isArray(input)) {
    for (const row of input) {
      const found = findNumericByKey(row, key, depth + 1);
      if (Number.isFinite(found)) return found;
    }
    return Number.NaN;
  }
  for (const value of Object.values(input)) {
    const found = findNumericByKey(value, key, depth + 1);
    if (Number.isFinite(found)) return found;
  }
  return Number.NaN;
}

function extractClassInputs(report: any) {
  const project = report?.projects?.Infring ?? {};
  const commandPathTotalMs = firstFinite([
    project?.rich_end_to_end_command_path_sample_total_ms,
    report?.rich_end_to_end_command_path_sample_total_ms,
    findNumericByKey(report, 'rich_end_to_end_command_path_sample_total_ms'),
  ]);
  const commandPathSamples = firstFinite([
    project?.rich_end_to_end_command_path_samples,
    report?.rich_end_to_end_command_path_samples,
    findNumericByKey(report, 'rich_end_to_end_command_path_samples'),
  ]);
  const realisticWorkloadMs =
    Number.isFinite(commandPathTotalMs) && Number.isFinite(commandPathSamples) && commandPathSamples > 0
      ? commandPathTotalMs / commandPathSamples
      : firstFinite([
          project?.cold_start_user_visible_ms,
          report?.cold_start_user_visible_ms,
          findNumericByKey(report, 'cold_start_user_visible_ms'),
        ]);

  return {
    kernelSharedOps: firstFinite([
      project?.kernel_shared_workload_ops_per_sec,
      report?.kernel_shared_workload_ops_per_sec,
      findNumericByKey(report, 'kernel_shared_workload_ops_per_sec'),
    ]),
    governedCommandPathOps: firstFinite([
      project?.rich_end_to_end_command_path_ops_per_sec,
      report?.rich_end_to_end_command_path_ops_per_sec,
      findNumericByKey(report, 'rich_end_to_end_command_path_ops_per_sec'),
    ]),
    realisticWorkloadMs,
    artifactSizeMb: firstFinite([
      project?.install_size_mb,
      report?.install_size_mb,
      findNumericByKey(report, 'install_size_mb'),
    ]),
  };
}

function round(value: number): number {
  return Number(value.toFixed(3));
}

function classifyTrend(direction: Direction, current: number, baseline: number): ClassMetric['trend'] {
  if (!Number.isFinite(current) || !Number.isFinite(baseline) || baseline <= 0) return 'unknown';
  if (current === baseline) return 'flat';
  if (direction === 'higher_is_better') return current > baseline ? 'improved' : 'regressed';
  return current < baseline ? 'improved' : 'regressed';
}

function rawDeltaPct(current: number, baseline: number): number | null {
  if (!Number.isFinite(current) || !Number.isFinite(baseline) || baseline <= 0) return null;
  return round(((current / baseline) - 1) * 100);
}

function improvementPct(direction: Direction, current: number, baseline: number): number | null {
  if (!Number.isFinite(current) || !Number.isFinite(baseline) || baseline <= 0) return null;
  if (direction === 'higher_is_better') {
    return round(((current / baseline) - 1) * 100);
  }
  return round(((baseline / current) - 1) * 100);
}

function renderMarkdown(payload: Payload): string {
  const lines: string[] = [];
  lines.push('# Benchmark Class Trends (Current)');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Strict: ${payload.strict ? 'true' : 'false'}`);
  lines.push(`Pass: ${payload.ok ? 'true' : 'false'}`);
  lines.push(`Current report: ${payload.current_report_path}`);
  lines.push(`Baseline report: ${payload.baseline_report_path}`);
  lines.push('');
  lines.push('## Class Trends');
  for (const row of payload.class_metrics) {
    lines.push(`- ${row.label}:`);
    lines.push(`  - metric: ${row.metric}`);
    lines.push(`  - direction: ${row.direction}`);
    lines.push(`  - current: ${row.current == null ? 'missing' : row.current}`);
    lines.push(`  - baseline: ${row.baseline == null ? 'missing' : row.baseline}`);
    lines.push(`  - raw_delta_pct: ${row.raw_delta_pct == null ? 'missing' : row.raw_delta_pct}`);
    lines.push(`  - improvement_pct: ${row.improvement_pct == null ? 'missing' : row.improvement_pct}`);
    lines.push(`  - trend: ${row.trend}`);
  }
  lines.push('');
  if (payload.missing_metrics.length > 0) {
    lines.push('## Missing Metrics');
    for (const missing of payload.missing_metrics) {
      lines.push(`- ${missing}`);
    }
    lines.push('');
  }
  return `${lines.join('\n')}\n`;
}

function toNullable(value: number): number | null {
  return Number.isFinite(value) ? round(value) : null;
}

function main(): void {
  const args = parseArgs(process.argv.slice(2));
  const currentPath = resolve(ROOT, args.currentReport);
  const baselinePath = resolve(ROOT, args.baselineReport);
  const outJsonPath = resolve(ROOT, args.outJson);
  const outMarkdownPath = resolve(ROOT, args.outMarkdown);

  const currentReport = readJson(currentPath);
  const baselineReport = readJson(baselinePath);

  const current = extractClassInputs(currentReport);
  const baseline = extractClassInputs(baselineReport);

  const classMetrics: ClassMetric[] = [
    {
      id: 'kernel_shared_workload',
      label: 'Kernel/shared workload',
      metric: 'kernel_shared_workload_ops_per_sec',
      direction: 'higher_is_better',
      current: toNullable(current.kernelSharedOps),
      baseline: toNullable(baseline.kernelSharedOps),
      raw_delta_pct: rawDeltaPct(current.kernelSharedOps, baseline.kernelSharedOps),
      improvement_pct: improvementPct('higher_is_better', current.kernelSharedOps, baseline.kernelSharedOps),
      trend: classifyTrend('higher_is_better', current.kernelSharedOps, baseline.kernelSharedOps),
      current_source: args.currentReport,
      baseline_source: args.baselineReport,
    },
    {
      id: 'governed_command_path',
      label: 'Governed command path',
      metric: 'rich_end_to_end_command_path_ops_per_sec',
      direction: 'higher_is_better',
      current: toNullable(current.governedCommandPathOps),
      baseline: toNullable(baseline.governedCommandPathOps),
      raw_delta_pct: rawDeltaPct(current.governedCommandPathOps, baseline.governedCommandPathOps),
      improvement_pct: improvementPct(
        'higher_is_better',
        current.governedCommandPathOps,
        baseline.governedCommandPathOps,
      ),
      trend: classifyTrend('higher_is_better', current.governedCommandPathOps, baseline.governedCommandPathOps),
      current_source: args.currentReport,
      baseline_source: args.baselineReport,
    },
    {
      id: 'realistic_workload',
      label: 'Realistic workload',
      metric: 'rich_end_to_end_command_path_avg_ms',
      direction: 'lower_is_better',
      current: toNullable(current.realisticWorkloadMs),
      baseline: toNullable(baseline.realisticWorkloadMs),
      raw_delta_pct: rawDeltaPct(current.realisticWorkloadMs, baseline.realisticWorkloadMs),
      improvement_pct: improvementPct('lower_is_better', current.realisticWorkloadMs, baseline.realisticWorkloadMs),
      trend: classifyTrend('lower_is_better', current.realisticWorkloadMs, baseline.realisticWorkloadMs),
      current_source: args.currentReport,
      baseline_source: args.baselineReport,
    },
    {
      id: 'artifact_size',
      label: 'Artifact size',
      metric: 'install_size_mb',
      direction: 'lower_is_better',
      current: toNullable(current.artifactSizeMb),
      baseline: toNullable(baseline.artifactSizeMb),
      raw_delta_pct: rawDeltaPct(current.artifactSizeMb, baseline.artifactSizeMb),
      improvement_pct: improvementPct('lower_is_better', current.artifactSizeMb, baseline.artifactSizeMb),
      trend: classifyTrend('lower_is_better', current.artifactSizeMb, baseline.artifactSizeMb),
      current_source: args.currentReport,
      baseline_source: args.baselineReport,
    },
  ];

  const missingMetrics = classMetrics
    .filter((row) => row.current == null || row.baseline == null)
    .map((row) => row.id);

  const payload: Payload = {
    ok: missingMetrics.length === 0,
    type: 'benchmark_class_trends',
    strict: args.strict,
    generated_at: new Date().toISOString(),
    current_report_path: args.currentReport,
    baseline_report_path: args.baselineReport,
    missing_metrics: missingMetrics,
    class_metrics: classMetrics,
  };

  ensureParent(outJsonPath);
  writeFileSync(outJsonPath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
  ensureParent(outMarkdownPath);
  writeFileSync(outMarkdownPath, renderMarkdown(payload), 'utf8');
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);

  const missingCurrentOrBaseline = !existsSync(currentPath) || !existsSync(baselinePath) || missingMetrics.length > 0;
  if (args.strict && missingCurrentOrBaseline) {
    process.exit(1);
  }
}

main();
