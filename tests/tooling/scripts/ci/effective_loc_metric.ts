#!/usr/bin/env tsx

import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { runCommand } from '../../lib/process.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type Language = 'rs' | 'ts' | 'tsx';

type EffectiveLocCounts = {
  files: number;
  nonblank_loc: number;
  files_by_language: Record<Language, number>;
  nonblank_loc_by_language: Record<Language, number>;
};

const METRIC_ID = 'effective_production_nonblank_loc_v1';
const DEFAULT_OUT = 'core/local/artifacts/effective_loc_metric_current.json';
const DEFAULT_MD = 'local/workspace/reports/EFFECTIVE_LOC_METRIC_CURRENT.md';

const INCLUDE_GLOBS = ['*.rs', '*.ts', '*.tsx'];
const EXCLUDE_PATHSPECS = [
  ':(exclude)docs/**',
  ':(exclude)local/**',
  ':(exclude).infring/**',
  ':(exclude)tests/**',
  ':(exclude)**/tests/**',
  ':(exclude)**/test/**',
  ':(exclude)**/__tests__/**',
  ':(exclude)**/vendor/**',
  ':(exclude)**/*.min.ts',
  ':(exclude)**/*.min.tsx',
  ':(exclude)**/*.d.ts',
  ':(exclude)**/*-tests.*',
  ':(exclude)**/*_test.*',
  ':(exclude)**/*.test.*',
  ':(exclude)**/*.spec.*',
];

function shellQuote(raw: string): string {
  return `'${String(raw).replace(/'/g, `'\"'\"'`)}'`;
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT, 400),
    markdownOutPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MD, 400),
    ref: cleanText(readFlag(argv, 'ref') || 'HEAD', 120),
    baseRef: cleanText(readFlag(argv, 'base-ref') || '', 120),
  };
}

function ensureRefExists(root: string, ref: string): boolean {
  if (!ref) return false;
  return runCommand(['git', 'rev-parse', '--verify', `${ref}^{commit}`], {
    cwd: root,
    timeoutSec: 20,
  }).ok;
}

function trackedFilesAtRef(root: string, ref: string): string[] {
  const result = runCommand(['git', 'ls-tree', '-r', '--name-only', ref], {
    cwd: root,
    timeoutSec: 45,
  });
  if (!result.ok) return [];
  return String(result.stdout || '')
    .split('\n')
    .map((row) => row.trim().replace(/\\/g, '/'))
    .filter(Boolean);
}

function languageForFile(relPath: string): Language | null {
  const lower = relPath.toLowerCase();
  if (lower.endsWith('.rs')) return 'rs';
  if (lower.endsWith('.tsx')) return 'tsx';
  if (lower.endsWith('.ts')) return 'ts';
  return null;
}

function isEffectiveProductionSource(relPath: string): boolean {
  const normalized = relPath.replace(/\\/g, '/');
  const lower = normalized.toLowerCase();
  const lang = languageForFile(lower);
  if (!lang) return false;

  if (
    lower.startsWith('docs/') ||
    lower.startsWith('local/') ||
    lower.startsWith('.infring/') ||
    lower.startsWith('tests/')
  ) {
    return false;
  }

  if (
    lower.includes('/tests/') ||
    lower.includes('/test/') ||
    lower.includes('/__tests__/') ||
    lower.includes('/vendor/')
  ) {
    return false;
  }

  if (lower.endsWith('.min.ts') || lower.endsWith('.min.tsx') || lower.endsWith('.d.ts')) {
    return false;
  }

  const base = path.basename(lower);
  if (
    base.includes('.test.') ||
    base.includes('.spec.') ||
    /(^|[._-])test(s)?([._-]|$)/.test(base)
  ) {
    return false;
  }

  return true;
}

function countNonblankAtRef(root: string, ref: string, includeGlobs: string[]): number {
  const includePart = includeGlobs.map((row) => shellQuote(row)).join(' ');
  const excludePart = EXCLUDE_PATHSPECS.map((row) => shellQuote(row)).join(' ');
  const cmd = `git grep -I -n . ${shellQuote(ref)} -- ${includePart} ${excludePart} | wc -l | awk '{print $1}'`;
  const result = runCommand(['bash', '-lc', cmd], {
    cwd: root,
    timeoutSec: 180,
  });
  if (!result.ok) return 0;
  const parsed = Number(String(result.stdout || '').trim());
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : 0;
}

function computeCounts(root: string, ref: string): EffectiveLocCounts {
  const files = trackedFilesAtRef(root, ref).filter(isEffectiveProductionSource);
  const filesByLanguage: Record<Language, number> = { rs: 0, ts: 0, tsx: 0 };
  for (const relPath of files) {
    const lang = languageForFile(relPath);
    if (lang) filesByLanguage[lang] += 1;
  }

  const nonblankByLanguage: Record<Language, number> = {
    rs: countNonblankAtRef(root, ref, ['*.rs']),
    ts: countNonblankAtRef(root, ref, ['*.ts']),
    tsx: countNonblankAtRef(root, ref, ['*.tsx']),
  };
  const nonblankTotal = nonblankByLanguage.rs + nonblankByLanguage.ts + nonblankByLanguage.tsx;

  return {
    files: files.length,
    nonblank_loc: nonblankTotal,
    files_by_language: filesByLanguage,
    nonblank_loc_by_language: nonblankByLanguage,
  };
}

function ratioPct(numerator: number, denominator: number): number {
  if (!Number.isFinite(numerator) || !Number.isFinite(denominator) || denominator <= 0) {
    return 0;
  }
  return Number(((numerator / denominator) * 100).toFixed(3));
}

function markdownForReport(report: any): string {
  const lines = [
    '# Effective Production LoC Metric',
    '',
    `- metric_id: ${report.metric_id}`,
    `- ref: ${report.ref}`,
    `- revision: ${report.revision}`,
    `- files: ${report.counts.files}`,
    `- nonblank_loc: ${report.counts.nonblank_loc}`,
    `- rs_nonblank_loc: ${report.counts.nonblank_loc_by_language.rs}`,
    `- ts_nonblank_loc: ${report.counts.nonblank_loc_by_language.ts}`,
    `- tsx_nonblank_loc: ${report.counts.nonblank_loc_by_language.tsx}`,
    `- rs_share_pct: ${report.composition.rs_share_pct}`,
    '',
    '## Canonical Definition',
    `- include_extensions: ${report.definition.include_extensions.join(', ')}`,
    `- exclude_pathspecs: ${report.definition.exclude_pathspecs.join(', ')}`,
  ];
  if (report.base_ref) {
    lines.push('');
    lines.push('## Delta');
    lines.push(`- base_ref: ${report.base_ref}`);
    lines.push(`- base_revision: ${report.base_revision}`);
    lines.push(`- delta_files: ${report.delta.files}`);
    lines.push(`- delta_nonblank_loc: ${report.delta.nonblank_loc}`);
    lines.push(`- delta_rs_nonblank_loc: ${report.delta.nonblank_loc_by_language.rs}`);
    lines.push(`- delta_ts_nonblank_loc: ${report.delta.nonblank_loc_by_language.ts}`);
    lines.push(`- delta_tsx_nonblank_loc: ${report.delta.nonblank_loc_by_language.tsx}`);
    lines.push(`- delta_rs_share_pct: ${report.delta.rs_share_pct}`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);

  if (!ensureRefExists(root, args.ref)) {
    return emitStructuredResult(
      {
        ok: false,
        type: 'effective_loc_metric',
        metric_id: METRIC_ID,
        error: 'invalid_ref',
        ref: args.ref,
      },
      {
        outPath: args.outPath,
        strict: args.strict,
        ok: false,
      },
    );
  }

  if (args.baseRef && !ensureRefExists(root, args.baseRef)) {
    return emitStructuredResult(
      {
        ok: false,
        type: 'effective_loc_metric',
        metric_id: METRIC_ID,
        error: 'invalid_base_ref',
        ref: args.ref,
        base_ref: args.baseRef,
      },
      {
        outPath: args.outPath,
        strict: args.strict,
        ok: false,
      },
    );
  }

  const counts = computeCounts(root, args.ref);
  const payload: any = {
    ok: true,
    type: 'effective_loc_metric',
    metric_id: METRIC_ID,
    generated_at: new Date().toISOString(),
    ref: args.ref,
    revision: currentRevision(root),
    definition: {
      include_extensions: ['.rs', '.ts', '.tsx'],
      line_type: 'nonblank',
      include_tracked_only: true,
      exclude_pathspecs: EXCLUDE_PATHSPECS,
    },
    counts,
    composition: {
      rs_share_pct: ratioPct(counts.nonblank_loc_by_language.rs, counts.nonblank_loc),
      ts_share_pct: ratioPct(counts.nonblank_loc_by_language.ts, counts.nonblank_loc),
      tsx_share_pct: ratioPct(counts.nonblank_loc_by_language.tsx, counts.nonblank_loc),
    },
    markdown_path: args.markdownOutPath,
    artifact_paths: [args.markdownOutPath],
  };

  if (args.baseRef) {
    const baseCounts = computeCounts(root, args.baseRef);
    payload.base_ref = args.baseRef;
    payload.base_revision = runCommand(['git', 'rev-parse', args.baseRef], {
      cwd: root,
      timeoutSec: 20,
    }).stdout.trim();
    payload.base_counts = baseCounts;
    payload.delta = {
      files: counts.files - baseCounts.files,
      nonblank_loc: counts.nonblank_loc - baseCounts.nonblank_loc,
      nonblank_loc_by_language: {
        rs: counts.nonblank_loc_by_language.rs - baseCounts.nonblank_loc_by_language.rs,
        ts: counts.nonblank_loc_by_language.ts - baseCounts.nonblank_loc_by_language.ts,
        tsx: counts.nonblank_loc_by_language.tsx - baseCounts.nonblank_loc_by_language.tsx,
      },
      rs_share_pct: Number(
        (
          ratioPct(counts.nonblank_loc_by_language.rs, counts.nonblank_loc) -
          ratioPct(baseCounts.nonblank_loc_by_language.rs, baseCounts.nonblank_loc)
        ).toFixed(3),
      ),
    };
  }

  writeTextArtifact(args.markdownOutPath, markdownForReport(payload));
  return emitStructuredResult(payload, {
    outPath: args.outPath,
    strict: args.strict,
    ok: true,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
