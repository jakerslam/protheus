#!/usr/bin/env node
/* eslint-disable no-console */
import { execSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_POLICY_PATH = 'client/runtime/config/shell_naming_policy.json';
const DEFAULT_OUT_JSON_PATH = 'core/local/artifacts/shell_naming_policy_guard_current.json';
const DEFAULT_OUT_MARKDOWN_PATH = 'local/workspace/reports/SHELL_NAMING_POLICY_GUARD_CURRENT.md';

function rel(p: string): string {
  return path.relative(ROOT, p).replace(/\\/g, '/');
}

function safeRegex(pattern: string, fallback: RegExp): RegExp {
  try {
    return new RegExp(pattern);
  } catch {
    return fallback;
  }
}

function loadPolicy(policyPath: string) {
  const abs = path.resolve(ROOT, policyPath);
  const parsed = JSON.parse(readFileSync(abs, 'utf8'));
  const includePrefixes = Array.isArray(parsed.include_prefixes)
    ? parsed.include_prefixes.map((v: unknown) => String(v).replace(/\\/g, '/'))
    : [];
  return {
    absPath: abs,
    includePrefixes,
    allowedUppercaseFilenames: new Set(
      Array.isArray(parsed.allowed_uppercase_filenames)
        ? parsed.allowed_uppercase_filenames.map((v: unknown) => String(v))
        : [],
    ),
    allowedUppercasePaths: new Set(
      Array.isArray(parsed.allowed_uppercase_paths)
        ? parsed.allowed_uppercase_paths.map((v: unknown) => String(v).replace(/\\/g, '/'))
        : [],
    ),
    allowedSpecialStems: new Set(
      Array.isArray(parsed.allowed_special_stems)
        ? parsed.allowed_special_stems.map((v: unknown) => String(v))
        : [],
    ),
    allowedDirectorySegments: new Set(
      Array.isArray(parsed.allowed_directory_segments)
        ? parsed.allowed_directory_segments.map((v: unknown) => String(v))
        : [],
    ),
    segmentRegex: safeRegex(
      String(parsed.segment_regex || ''),
      /^[a-z0-9]+(?:[._-][a-z0-9]+)*$/,
    ),
    stemRegex: safeRegex(
      String(parsed.stem_regex || ''),
      /^[a-z0-9]+(?:[._-][a-z0-9]+)*$/,
    ),
    bannedGenericCodeStems: new Set(
      Array.isArray(parsed.banned_generic_code_stems)
        ? parsed.banned_generic_code_stems.map((v: unknown) => String(v).toLowerCase())
        : [],
    ),
    codeExtensions: new Set(
      Array.isArray(parsed.code_extensions)
        ? parsed.code_extensions.map((v: unknown) => String(v).toLowerCase())
        : [],
    ),
  };
}

function listTrackedFiles(includePrefixes: string[]): string[] {
  let raw = '';
  try {
    raw = execSync('git ls-files', { cwd: ROOT, encoding: 'utf8' });
  } catch {
    return [];
  }
  const files = raw
    .split('\n')
    .map((line) => line.trim().replace(/\\/g, '/'))
    .filter(Boolean);
  if (includePrefixes.length === 0) return files;
  return files.filter((file) => includePrefixes.some((prefix) => file === prefix || file.startsWith(`${prefix}/`)));
}

function buildMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Shell Naming Policy Guard');
  lines.push('');
  lines.push(`- Generated at: ${report.generated_at}`);
  lines.push(`- Revision: ${report.revision}`);
  lines.push(`- Strict mode: ${report.strict ? 'true' : 'false'}`);
  lines.push(`- Scanned files: ${report.scanned_files}`);
  lines.push(`- Violations: ${report.violation_count}`);
  lines.push(`- Pass: ${report.ok ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Violation Counts');
  lines.push('');
  lines.push('| Rule | Count |');
  lines.push('| --- | ---: |');
  for (const [rule, count] of Object.entries(report.violation_counts || {})) {
    lines.push(`| ${rule} | ${count} |`);
  }
  if (!Object.keys(report.violation_counts || {}).length) {
    lines.push('| (none) | 0 |');
  }
  lines.push('');
  lines.push('## Sample Violations');
  lines.push('');
  lines.push('| Rule | Path | Segment |');
  lines.push('| --- | --- | --- |');
  const sample = Array.isArray(report.violations) ? report.violations.slice(0, 60) : [];
  for (const row of sample) {
    lines.push(`| ${String(row.rule || '')} | ${String(row.path || '')} | ${String(row.segment || '')} |`);
  }
  if (!sample.length) {
    lines.push('| (none) | - | - |');
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function currentRevision(): string {
  try {
    return execSync('git rev-parse HEAD', { cwd: ROOT, encoding: 'utf8' }).trim();
  } catch {
    return 'unknown';
  }
}

function main() {
  const argv = process.argv.slice(2);
  const strictOut = parseStrictOutArgs(argv, {
    strict: false,
    out: DEFAULT_OUT_JSON_PATH,
  });
  const outJson = cleanText(readFlag(argv, 'out-json') || strictOut.out || DEFAULT_OUT_JSON_PATH, 400);
  const outMarkdown = cleanText(
    readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN_PATH,
    400,
  );
  const policyPath = cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY_PATH, 400);

  const policy = loadPolicy(policyPath);
  const files = listTrackedFiles(policy.includePrefixes).sort();
  const violations: Array<{ rule: string; path: string; segment?: string }> = [];
  let skippedDotfiles = 0;

  for (const file of files) {
    const segments = file.split('/');
    const fileName = segments[segments.length - 1] || '';
    const directorySegments = segments.slice(0, -1);
    const fileIsUppercaseAllowed =
      policy.allowedUppercaseFilenames.has(fileName) || policy.allowedUppercasePaths.has(file);

    if (/\s/.test(file)) {
      violations.push({ rule: 'no_whitespace_in_paths', path: file });
    }

    for (const seg of directorySegments) {
      if (policy.allowedDirectorySegments.has(seg)) {
        continue;
      }
      if (!policy.segmentRegex.test(seg)) {
        violations.push({ rule: 'directory_segment_style', path: file, segment: seg });
      }
      if (/[A-Z]/.test(seg)) {
        violations.push({ rule: 'no_uppercase_directory_segments', path: file, segment: seg });
      }
    }

    const ext = path.extname(fileName).replace(/^\./, '').toLowerCase();
    const stem = fileName.replace(/\.[^.]+$/, '');

    if (fileName.startsWith('.')) {
      skippedDotfiles += 1;
      continue;
    }

    if (!fileIsUppercaseAllowed && /[A-Z]/.test(fileName)) {
      violations.push({ rule: 'no_uppercase_filenames', path: file, segment: fileName });
    }

    const isSpecialStem = stem.startsWith('+') && policy.allowedSpecialStems.has(stem);
    if (!fileIsUppercaseAllowed && !isSpecialStem && !policy.stemRegex.test(stem)) {
      violations.push({ rule: 'filename_stem_style', path: file, segment: stem });
    }

    if (policy.codeExtensions.has(ext) && policy.bannedGenericCodeStems.has(stem.toLowerCase())) {
      violations.push({ rule: 'banned_generic_code_stem', path: file, segment: stem });
    }
  }

  const violationCounts: Record<string, number> = {};
  for (const row of violations) {
    violationCounts[row.rule] = (violationCounts[row.rule] || 0) + 1;
  }

  const report = {
    ok: violations.length === 0,
    type: 'shell_naming_policy_guard',
    generated_at: new Date().toISOString(),
    strict: strictOut.strict,
    revision: currentRevision(),
    policy_path: rel(policy.absPath),
    scanned_files: files.length,
    skipped_dotfiles: skippedDotfiles,
    violation_count: violations.length,
    violation_counts: Object.fromEntries(
      Object.entries(violationCounts).sort((a, b) => String(a[0]).localeCompare(String(b[0]))),
    ),
    violations,
  };

  writeTextArtifact(path.resolve(ROOT, outMarkdown), buildMarkdown(report));
  const exitCode = emitStructuredResult(report, {
    outPath: path.resolve(ROOT, outJson),
    strict: strictOut.strict,
    ok: report.ok,
  });
  if (exitCode !== 0) process.exit(exitCode);
}

main();
