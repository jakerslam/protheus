#!/usr/bin/env node
/* eslint-disable no-console */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

const ROOT = resolve(__dirname, '..', '..', '..', '..');
const OUT_JSON = 'core/local/artifacts/benchmark_public_audit_current.json';
const OUT_MD = 'local/workspace/reports/BENCHMARK_PUBLIC_AUDIT_CURRENT.md';

const README_PATH = 'README.md';
const COMPETITIVE_MATRIX_README_PATH = 'benchmarks/competitive_matrix/README.md';
const PUBLIC_BENCHMARKS_PATH = 'docs/client/PUBLIC_BENCHMARKS.md';
const CANONICAL_REPORT_PATH = 'docs/client/reports/benchmark_matrix_run_latest.json';
const RELEASE_WORKFLOW_PATH = '.github/workflows/release.yml';

const REQUIRED_REPRO_COMMAND = 'npm run -s ops:benchmark:repro';

const BANNED_PUBLIC_ARTIFACT_PATHS = [
  'local/state/ops/competitive_benchmark_matrix/latest.json',
  'client/runtime/local/state/ops/competitive_benchmark_matrix/latest.json'
];

type Options = {
  strict: boolean;
};

type AuditPayload = {
  ok: boolean;
  type: string;
  strict: boolean;
  generated_at: string;
  canonical_report_path: string;
  checked_surfaces: string[];
  violations: string[];
  notes: string[];
};

function parseBool(raw: string, fallback: boolean): boolean {
  const value = String(raw || '').trim().toLowerCase();
  if (!value) return fallback;
  if (['1', 'true', 'yes', 'on'].includes(value)) return true;
  if (['0', 'false', 'no', 'off'].includes(value)) return false;
  return fallback;
}

function parseArgs(argv: string[]): Options {
  const out: Options = { strict: false };
  for (const raw of argv) {
    const arg = String(raw || '').trim();
    if (!arg) continue;
    if (arg === '--strict' || arg === '--strict=1') {
      out.strict = true;
      continue;
    }
    if (arg.startsWith('--strict=')) {
      out.strict = parseBool(arg.slice('--strict='.length), out.strict);
    }
  }
  return out;
}

function readText(relPath: string): string {
  return readFileSync(resolve(ROOT, relPath), 'utf8');
}

function readJson(relPath: string): any {
  return JSON.parse(readText(relPath));
}

function ensureParent(relPath: string): void {
  mkdirSync(dirname(resolve(ROOT, relPath)), { recursive: true });
}

function writeJson(relPath: string, payload: unknown): void {
  ensureParent(relPath);
  writeFileSync(resolve(ROOT, relPath), `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function writeText(relPath: string, body: string): void {
  ensureParent(relPath);
  writeFileSync(resolve(ROOT, relPath), body, 'utf8');
}

function findMarkdownLinks(markdown: string): string[] {
  const links: string[] = [];
  const re = /\[[^\]]*]\(([^)]+)\)/g;
  let match: RegExpExecArray | null = null;
  while ((match = re.exec(markdown)) != null) {
    links.push(String(match[1] || '').trim());
  }
  return links;
}

function isExternalLink(link: string): boolean {
  return (
    link.startsWith('http://') ||
    link.startsWith('https://') ||
    link.startsWith('mailto:') ||
    link.startsWith('#')
  );
}

function asFinite(value: unknown): number | null {
  const num = Number(value);
  if (!Number.isFinite(num)) return null;
  return num;
}

function toMarkdown(payload: AuditPayload): string {
  const lines: string[] = [];
  lines.push('# Benchmark Public Audit (Current)');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Strict: ${payload.strict ? 'true' : 'false'}`);
  lines.push(`Pass: ${payload.ok ? 'true' : 'false'}`);
  lines.push(`Canonical Report: ${payload.canonical_report_path}`);
  lines.push('');
  lines.push('## Surfaces');
  for (const surface of payload.checked_surfaces) {
    lines.push(`- ${surface}`);
  }
  lines.push('');
  if (payload.notes.length > 0) {
    lines.push('## Notes');
    for (const note of payload.notes) {
      lines.push(`- ${note}`);
    }
    lines.push('');
  }
  if (payload.violations.length > 0) {
    lines.push('## Violations');
    for (const violation of payload.violations) {
      lines.push(`- ${violation}`);
    }
    lines.push('');
  }
  return `${lines.join('\n')}\n`;
}

function hasReproCommand(body: string): boolean {
  const text = String(body || '');
  if (text.includes(REQUIRED_REPRO_COMMAND)) return true;
  return (
    text.includes('npm run -s ops:benchmark:refresh') &&
    text.includes('npm run -s ops:benchmark:sanity') &&
    text.includes('npm run -s ops:benchmark:public-audit')
  );
}

function main(): void {
  const options = parseArgs(process.argv.slice(2));
  const checkedSurfaces = [
    README_PATH,
    COMPETITIVE_MATRIX_README_PATH,
    PUBLIC_BENCHMARKS_PATH,
    RELEASE_WORKFLOW_PATH,
    CANONICAL_REPORT_PATH
  ];
  const violations: string[] = [];
  const notes: string[] = [];

  for (const relPath of checkedSurfaces) {
    if (!existsSync(resolve(ROOT, relPath))) {
      violations.push(`missing_surface:${relPath}`);
    }
  }

  const readme = existsSync(resolve(ROOT, README_PATH)) ? readText(README_PATH) : '';
  const competitiveReadme = existsSync(resolve(ROOT, COMPETITIVE_MATRIX_README_PATH))
    ? readText(COMPETITIVE_MATRIX_README_PATH)
    : '';
  const publicBench = existsSync(resolve(ROOT, PUBLIC_BENCHMARKS_PATH))
    ? readText(PUBLIC_BENCHMARKS_PATH)
    : '';
  const releaseWorkflow = existsSync(resolve(ROOT, RELEASE_WORKFLOW_PATH))
    ? readText(RELEASE_WORKFLOW_PATH)
    : '';

  if (!readme.includes(CANONICAL_REPORT_PATH)) {
    violations.push(`readme_missing_canonical_benchmark_link:${CANONICAL_REPORT_PATH}`);
  }
  if (!competitiveReadme.includes(CANONICAL_REPORT_PATH)) {
    violations.push(
      `competitive_matrix_readme_missing_canonical_benchmark_link:${CANONICAL_REPORT_PATH}`
    );
  }
  if (!publicBench.includes(CANONICAL_REPORT_PATH)) {
    violations.push(`public_benchmarks_missing_canonical_benchmark_link:${CANONICAL_REPORT_PATH}`);
  }
  if (!releaseWorkflow.includes(CANONICAL_REPORT_PATH)) {
    violations.push(`release_workflow_missing_canonical_benchmark_asset:${CANONICAL_REPORT_PATH}`);
  }

  if (!hasReproCommand(readme)) {
    violations.push(`readme_missing_repro_command:${REQUIRED_REPRO_COMMAND}`);
  }
  if (!hasReproCommand(competitiveReadme)) {
    violations.push(`competitive_matrix_readme_missing_repro_command:${REQUIRED_REPRO_COMMAND}`);
  }
  if (!hasReproCommand(publicBench)) {
    violations.push(`public_benchmarks_missing_repro_command:${REQUIRED_REPRO_COMMAND}`);
  }

  const publicSurfaces: Array<[string, string]> = [
    [README_PATH, readme],
    [COMPETITIVE_MATRIX_README_PATH, competitiveReadme],
    [PUBLIC_BENCHMARKS_PATH, publicBench]
  ];
  for (const [surfacePath, surfaceBody] of publicSurfaces) {
    for (const banned of BANNED_PUBLIC_ARTIFACT_PATHS) {
      if (surfaceBody.includes(banned)) {
        violations.push(`non_public_artifact_path_in_surface:${surfacePath}:${banned}`);
      }
    }
    const links = findMarkdownLinks(surfaceBody);
    for (const link of links) {
      if (isExternalLink(link)) continue;
      const normalized = link.replace(/\\/g, '/');
      if (normalized.startsWith('local/state/') || normalized.startsWith('client/runtime/local/')) {
        violations.push(`local_runtime_link_in_public_surface:${surfacePath}:${normalized}`);
      }
      if (normalized.endsWith('.json') && !existsSync(resolve(ROOT, normalized))) {
        violations.push(`missing_json_link_target:${surfacePath}:${normalized}`);
      }
    }
  }

  if (existsSync(resolve(ROOT, CANONICAL_REPORT_PATH))) {
    let report: any = null;
    try {
      report = readJson(CANONICAL_REPORT_PATH);
    } catch (error) {
      violations.push(
        `canonical_benchmark_report_parse_failed:${String(
          (error as Error)?.message || error || 'unknown'
        )}`
      );
    }
    if (report) {
      const reportType = String(report?.type || '').trim();
      if (!reportType.includes('benchmark_matrix')) {
        violations.push(`canonical_report_unexpected_type:${reportType || 'missing'}`);
      }
      const projects = report?.projects && typeof report.projects === 'object' ? report.projects : {};
      const rich = projects['InfRing (rich)'] || projects.Infring || report?.infring_measured;
      const pure = projects['InfRing (pure)'] || report?.pure_workspace_measured;
      const tiny = projects['InfRing (tiny-max)'] || report?.pure_workspace_tiny_max_measured;
      if (!rich) violations.push('canonical_report_missing_project:InfRing (rich)');
      if (!pure) violations.push('canonical_report_missing_project:InfRing (pure)');
      if (!tiny) violations.push('canonical_report_missing_project:InfRing (tiny-max)');

      const requiredRichMetrics = ['cold_start_ms', 'idle_memory_mb', 'install_size_mb', 'tasks_per_sec'];
      for (const metric of requiredRichMetrics) {
        const val =
          rich && typeof rich === 'object'
            ? asFinite((rich as Record<string, unknown>)[metric])
            : null;
        if (val == null) {
          violations.push(`canonical_report_missing_metric:InfRing (rich):${metric}`);
        }
      }
      const benchmarkValidationOk = report?.benchmark_validation?.ok;
      notes.push(`canonical_report_benchmark_validation_ok=${String(benchmarkValidationOk)}`);
    }
  }

  const payload: AuditPayload = {
    ok: violations.length === 0,
    type: 'benchmark_public_audit',
    strict: options.strict,
    generated_at: new Date().toISOString(),
    canonical_report_path: CANONICAL_REPORT_PATH,
    checked_surfaces: checkedSurfaces,
    violations,
    notes
  };

  writeJson(OUT_JSON, payload);
  writeText(OUT_MD, toMarkdown(payload));

  console.log(JSON.stringify(payload, null, 2));
  if (options.strict && !payload.ok) {
    process.exit(1);
  }
}

main();
