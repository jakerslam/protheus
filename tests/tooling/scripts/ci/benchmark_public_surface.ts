#!/usr/bin/env node
/* eslint-disable no-console */
import { basename, relative, resolve } from 'node:path';

export const README_BENCHMARK_START = '<!-- BEGIN: benchmark-snapshot -->';
export const README_BENCHMARK_END = '<!-- END: benchmark-snapshot -->';
export const CANONICAL_THROUGHPUT_METRIC = 'tasks_per_sec';

const ABSOLUTE_PATH_PATTERNS: RegExp[] = [
  /^\/(Users|home|var|tmp|private|opt)\//i,
  /^[a-zA-Z]:[\\/]/,
  /^\\\\/,
];

const LEGACY_ALIAS_PATTERNS: RegExp[] = [
  /\bprotheus-pure-workspace\b/gi,
  /\bprotheus-ops\b/gi,
  /\bprotheusd\b/gi,
  /\bprotheusctl\b/gi,
  /\bprotheus\b/gi,
];

const LEGACY_ALIAS_REPLACEMENTS: Array<{ pattern: RegExp; replacement: string }> = [
  { pattern: /\bprotheus-pure-workspace\b/gi, replacement: 'infring-pure-workspace' },
  { pattern: /\bprotheus-ops\b/gi, replacement: 'infring-ops' },
  { pattern: /\bprotheusd\b/gi, replacement: 'infringd' },
  { pattern: /\bprotheusctl\b/gi, replacement: 'infringctl' },
  { pattern: /\bprotheus\b/gi, replacement: 'infring' },
];

function normalizePathString(raw: string): string {
  return String(raw || '').trim().replace(/\\/g, '/');
}

function containsLegacyAlias(raw: string): boolean {
  const value = String(raw || '');
  return LEGACY_ALIAS_PATTERNS.some((pattern) => pattern.test(value));
}

function normalizeLegacyAliases(raw: string): string {
  let value = String(raw || '');
  for (const row of LEGACY_ALIAS_REPLACEMENTS) {
    value = value.replace(row.pattern, row.replacement);
  }
  return value;
}

function asFiniteNumber(value: unknown): number | null {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function formatFixed(value: unknown, digits: number): string {
  const parsed = asFiniteNumber(value);
  if (parsed == null) return 'n/a';
  return parsed.toFixed(digits);
}

function formatCount(value: unknown, fallbackZero = false): string {
  const parsed = asFiniteNumber(value);
  if (parsed == null) return fallbackZero ? '0' : 'n/a';
  return String(Math.round(parsed));
}

function formatThroughput(value: unknown): string {
  const parsed = asFiniteNumber(value);
  if (parsed == null) return 'n/a';
  return Number(parsed).toLocaleString('en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

function normalizeProjects(report: any): Record<string, any> {
  const projects =
    report && report.projects && typeof report.projects === 'object' ? { ...report.projects } : {};
  if (!projects['InfRing (rich)'] && projects.Infring) {
    projects['InfRing (rich)'] = projects.Infring;
  }
  if (!projects.Infring && projects['InfRing (rich)']) {
    projects.Infring = projects['InfRing (rich)'];
  }
  return projects;
}

function resolveRichProject(report: any): { label: string; payload: any } {
  const projects = normalizeProjects(report);
  if (projects['InfRing (rich)']) {
    return { label: 'InfRing (rich)', payload: projects['InfRing (rich)'] };
  }
  if (projects.Infring) {
    return { label: 'Infring', payload: projects.Infring };
  }
  return {
    label: 'InfRing (rich)',
    payload: report?.infring_measured && typeof report.infring_measured === 'object' ? report.infring_measured : {},
  };
}

function resolveProject(report: any, name: string, fallback: any): any {
  const projects = normalizeProjects(report);
  if (projects[name] && typeof projects[name] === 'object') return projects[name];
  return fallback && typeof fallback === 'object' ? fallback : {};
}

function extractCvTolerancePct(report: any): number | null {
  const checks = Array.isArray(report?.benchmark_validation?.checks)
    ? report.benchmark_validation.checks
    : [];
  for (const check of checks) {
    if (String(check?.id || '').trim() !== 'shared_cv_within_tolerance') continue;
    const detail = check?.detail && typeof check.detail === 'object' ? check.detail : {};
    const tolerance = asFiniteNumber((detail as Record<string, unknown>).tolerance_pct);
    if (tolerance != null) return tolerance;
  }
  return null;
}

function competitorRows(report: any): Array<{ label: string; payload: any }> {
  const projects = normalizeProjects(report);
  const ordered = ['Infring', 'LangGraph', 'AutoGen', 'CrewAI', 'OpenHands', 'Workflow Graph'];
  const excluded = new Set(['InfRing (rich)', 'InfRing (pure)', 'InfRing (tiny-max)']);
  const seen = new Set<string>();
  const rows: Array<{ label: string; payload: any }> = [];

  for (const name of ordered) {
    if (seen.has(name)) continue;
    const payload = projects[name];
    if (!payload || typeof payload !== 'object') continue;
    rows.push({ label: name, payload });
    seen.add(name);
  }

  const extras = Object.keys(projects)
    .filter((name) => !seen.has(name))
    .filter((name) => !excluded.has(name))
    .filter((name) => typeof projects[name] === 'object')
    .sort((a, b) => a.localeCompare(b));
  for (const name of extras) {
    rows.push({ label: name, payload: projects[name] });
  }

  return rows;
}

function renderMetricRows(report: any): string[] {
  const rich = resolveRichProject(report);
  const pure = resolveProject(report, 'InfRing (pure)', report?.pure_workspace_measured);
  const tiny = resolveProject(report, 'InfRing (tiny-max)', report?.pure_workspace_tiny_max_measured);
  const richEngineMs =
    rich.payload?.cold_start_engine_init_ms ??
    rich.payload?.engine_start_ms ??
    null;
  const richOrchestrationMs =
    rich.payload?.cold_start_orchestration_ms ??
    rich.payload?.gateway_supervisor_orchestration_ms ??
    null;
  const richKernelReadyMs =
    rich.payload?.kernel_ready_ms ??
    rich.payload?.engine_start_ms ??
    null;
  const richGatewayReadyMs =
    rich.payload?.gateway_ready_ms ??
    (Number.isFinite(Number(richKernelReadyMs)) && Number.isFinite(Number(richOrchestrationMs))
      ? Number(richKernelReadyMs) + Number(richOrchestrationMs)
      : null);
  const richDashboardInteractiveMs =
    rich.payload?.dashboard_interactive_ms ??
    rich.payload?.rich_cold_start_total_ms ??
    rich.payload?.cold_start_ms ??
    null;
  return [
    '| Metric | Rich | Pure (`InfRing (pure)`) | Tiny-Max (`InfRing (tiny-max)`) |',
    '|---|---:|---:|---:|',
    `| Cold start (user-visible) | ${formatFixed(rich.payload?.cold_start_ms, 3)} ms | ${formatFixed(pure?.cold_start_ms, 3)} ms | ${formatFixed(tiny?.cold_start_ms, 3)} ms |`,
    `| Cold start (engine init micro) | ${formatFixed(richEngineMs, 3)} ms | n/a | n/a |`,
    `| Cold start (orchestration component) | ${formatFixed(richOrchestrationMs, 3)} ms | n/a | n/a |`,
    `| Kernel ready | ${formatFixed(richKernelReadyMs, 3)} ms | n/a | n/a |`,
    `| Gateway ready | ${formatFixed(richGatewayReadyMs, 3)} ms | n/a | n/a |`,
    `| Dashboard interactive | ${formatFixed(richDashboardInteractiveMs, 3)} ms | n/a | n/a |`,
    `| Idle memory | ${formatFixed(rich.payload?.idle_memory_mb, 3)} MB | ${formatFixed(pure?.idle_memory_mb, 3)} MB | ${formatFixed(tiny?.idle_memory_mb, 3)} MB |`,
    `| Install artifact size | ${formatFixed(rich.payload?.install_size_mb, 3)} MB | ${formatFixed(pure?.install_size_mb, 3)} MB | ${formatFixed(tiny?.install_size_mb, 3)} MB |`,
    `| Throughput (${CANONICAL_THROUGHPUT_METRIC}) | ${formatThroughput(rich.payload?.[CANONICAL_THROUGHPUT_METRIC])} ops/sec | ${formatThroughput(pure?.[CANONICAL_THROUGHPUT_METRIC])} ops/sec | ${formatThroughput(tiny?.[CANONICAL_THROUGHPUT_METRIC])} ops/sec |`,
    `| Security systems | ${formatCount(rich.payload?.security_systems)} | ${formatCount(pure?.security_systems, true)} | ${formatCount(tiny?.security_systems, true)} |`,
    `| Channel adapters | ${formatCount(rich.payload?.channel_adapters)} | ${formatCount(pure?.channel_adapters, true)} | ${formatCount(tiny?.channel_adapters, true)} |`,
    `| LLM providers | ${formatCount(rich.payload?.llm_providers)} | ${formatCount(pure?.llm_providers, true)} | ${formatCount(tiny?.llm_providers, true)} |`,
    `| Data channels | ${formatCount(rich.payload?.data_channels)} | ${formatCount(pure?.data_channels, true)} | ${formatCount(tiny?.data_channels, true)} |`,
    `| Plugin marketplace checks | ${formatCount(rich.payload?.plugin_marketplace_checks)} | ${formatCount(pure?.plugin_marketplace_checks, true)} | ${formatCount(tiny?.plugin_marketplace_checks, true)} |`,
  ];
}

function renderCompetitorRows(report: any): string[] {
  const rows = competitorRows(report);
  const output = ['| Project | Cold Start (ms) | Idle Memory (MB) | Install Size (MB) | Throughput (ops/sec) |', '|---|---:|---:|---:|---:|'];
  for (const row of rows) {
    output.push(
      `| ${row.label} | ${formatFixed(row.payload?.cold_start_ms, 3)} | ${formatFixed(row.payload?.idle_memory_mb, 3)} | ${formatFixed(row.payload?.install_size_mb, 3)} | ${formatThroughput(row.payload?.[CANONICAL_THROUGHPUT_METRIC])} |`,
    );
  }
  return output;
}

export function renderBenchmarkSnapshotMarkdown(report: any): string {
  const preflightEnabled = report?.benchmark_preflight?.enabled;
  const benchmarkValidationOk = report?.benchmark_validation?.ok;
  const sampleCvPct = asFiniteNumber(report?.benchmark_validation?.sample_cv_pct);
  const tolerancePct = extractCvTolerancePct(report);
  const artifactTimestamp = String(report?.ts || report?.generated_at || 'n/a');
  const metricRows = renderMetricRows(report);
  const comparisonRows = renderCompetitorRows(report);

  const lines: string[] = [];
  lines.push('## Performance Snapshot (Latest Artifact)');
  lines.push('');
  lines.push('Latest benchmark source:');
  lines.push('');
  lines.push('- [`docs/client/reports/benchmark_matrix_run_latest.json`](docs/client/reports/benchmark_matrix_run_latest.json)');
  lines.push('');
  lines.push(`Canonical throughput metric: \`${CANONICAL_THROUGHPUT_METRIC}\``);
  lines.push('');
  lines.push('Current measured rows in that artifact:');
  lines.push('');
  lines.push(...metricRows);
  lines.push('');
  lines.push('Preflight metadata in the same artifact:');
  lines.push('');
  lines.push(`- \`benchmark_preflight.enabled = ${String(preflightEnabled)}\``);
  lines.push(`- \`benchmark_validation.ok = ${String(benchmarkValidationOk)}\``);
  lines.push(`- \`sample_cv_pct = ${sampleCvPct == null ? 'n/a' : sampleCvPct.toFixed(2)}\`${tolerancePct == null ? '' : ` (tolerance \`${tolerancePct}\`)`}`);
  lines.push(`- Artifact timestamp: \`${artifactTimestamp}\``);
  lines.push('');
  lines.push('Current nuance:');
  lines.push('');
  lines.push('- Public benchmark summaries are generated from the canonical artifact during refresh and verified by `ops:benchmark:public-audit`.');
  lines.push('- Reproducibility commands are listed below; claims should match the linked JSON artifact exactly.');
  lines.push('');
  lines.push('### Competitor Comparison (Latest Matrix)');
  lines.push('');
  lines.push('Source: [`docs/client/reports/benchmark_matrix_run_latest.json`](docs/client/reports/benchmark_matrix_run_latest.json)');
  lines.push('');
  lines.push(...comparisonRows);
  lines.push('');
  lines.push('Refresh commands:');
  lines.push('');
  lines.push('```bash');
  lines.push('npm run -s ops:benchmark:refresh');
  lines.push('npm run -s ops:benchmark:sanity');
  lines.push('npm run -s ops:benchmark:public-audit');
  lines.push('npm run -s ops:benchmark:repro');
  lines.push('```');
  return `${lines.join('\n')}\n`;
}

export function renderBenchmarkSnapshotBlock(report: any): string {
  return `${README_BENCHMARK_START}\n${renderBenchmarkSnapshotMarkdown(report)}${README_BENCHMARK_END}\n`;
}

export function extractBenchmarkSnapshotBlock(readme: string): string | null {
  const source = String(readme || '');
  const start = source.indexOf(README_BENCHMARK_START);
  const end = source.indexOf(README_BENCHMARK_END);
  if (start === -1 || end === -1 || end < start) return null;
  return source.slice(start, end + README_BENCHMARK_END.length).trimEnd();
}

export function upsertBenchmarkSnapshotBlock(readme: string, block: string): string {
  const source = String(readme || '');
  const start = source.indexOf(README_BENCHMARK_START);
  const end = source.indexOf(README_BENCHMARK_END);
  if (start === -1 || end === -1 || end < start) {
    throw new Error('readme_benchmark_markers_missing');
  }
  const next = `${source.slice(0, start)}${block.trimEnd()}\n${source.slice(end + README_BENCHMARK_END.length)}`;
  return next;
}

function looksLikeAbsolutePath(value: string): boolean {
  const normalized = normalizePathString(value);
  if (!normalized) return false;
  if (normalized.includes('/Users/')) return true;
  if (normalized.includes('/home/')) return true;
  if (normalized.includes('\\Users\\')) return true;
  return ABSOLUTE_PATH_PATTERNS.some((pattern) => pattern.test(normalized));
}

function sanitizePathValue(value: string, root: string): string {
  const normalized = normalizePathString(value);
  if (!looksLikeAbsolutePath(normalized)) return normalizeLegacyAliases(normalized);
  const absRoot = normalizePathString(resolve(root));
  if (normalized.startsWith(`${absRoot}/`)) {
    return normalizeLegacyAliases(relative(absRoot, normalized).replace(/\\/g, '/'));
  }
  const targetIdx = normalized.indexOf('/target/');
  if (targetIdx >= 0) {
    return normalizeLegacyAliases(normalized.slice(targetIdx + 1));
  }
  const localIdx = normalized.indexOf('/local/');
  if (localIdx >= 0) {
    return normalizeLegacyAliases(normalized.slice(localIdx + 1));
  }
  return normalizeLegacyAliases(`<redacted>/${basename(normalized)}`);
}

function deepSanitize(value: any, root: string): any {
  if (Array.isArray(value)) {
    return value.map((item) => deepSanitize(item, root));
  }
  if (value && typeof value === 'object') {
    const out: Record<string, unknown> = {};
    for (const [key, nested] of Object.entries(value)) {
      out[key] = deepSanitize(nested, root);
    }
    return out;
  }
  if (typeof value === 'string') {
    return normalizeLegacyAliases(sanitizePathValue(value, root));
  }
  return value;
}

export function sanitizePublicBenchmarkReport(report: any, root: string): any {
  return deepSanitize(report, root);
}

export function collectBenchmarkPathLeaks(payload: any): string[] {
  const leaks: string[] = [];
  function walk(value: any, path: string): void {
    if (Array.isArray(value)) {
      value.forEach((item, index) => walk(item, `${path}[${index}]`));
      return;
    }
    if (value && typeof value === 'object') {
      for (const [key, nested] of Object.entries(value)) {
        const nextPath = path ? `${path}.${key}` : key;
        walk(nested, nextPath);
      }
      return;
    }
    if (typeof value === 'string' && looksLikeAbsolutePath(value)) {
      leaks.push(path);
    }
  }
  walk(payload, '');
  return leaks;
}

export function collectBenchmarkAliasLeaks(payload: any): string[] {
  const leaks: string[] = [];
  function walk(value: any, path: string): void {
    if (Array.isArray(value)) {
      value.forEach((item, index) => walk(item, `${path}[${index}]`));
      return;
    }
    if (value && typeof value === 'object') {
      for (const [key, nested] of Object.entries(value)) {
        const nextPath = path ? `${path}.${key}` : key;
        walk(nested, nextPath);
      }
      return;
    }
    if (typeof value === 'string' && containsLegacyAlias(value)) {
      leaks.push(path);
    }
  }
  walk(payload, '');
  return leaks;
}
