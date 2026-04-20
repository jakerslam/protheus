#!/usr/bin/env node
/* eslint-disable no-console */
import { execSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON_PATH = 'core/local/artifacts/dashboard_segment_ghost_file_guard_current.json';
const DEFAULT_OUT_MARKDOWN_PATH = 'local/workspace/reports/DASHBOARD_SEGMENT_GHOST_FILE_GUARD_CURRENT.md';
const GHOST_STEM_PATTERN = /(?:^|[._-])(zz|bak|backup|orig|rej|tmp|temp|old)(?:[._-]|$)/i;

function rel(filePath: string): string {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function listTrackedFiles(): string[] {
  try {
    return execSync('git ls-files', { cwd: ROOT, encoding: 'utf8' })
      .split('\n')
      .map((line) => line.trim().replace(/\\/g, '/'))
      .filter(Boolean)
      .sort((a, b) => a.localeCompare(b, 'en'));
  } catch {
    return [];
  }
}

function isSegmentPartPath(filePath: string): boolean {
  return filePath.includes('.parts/') && fs.existsSync(path.resolve(ROOT, filePath));
}

function isGhostSegmentPartFile(filePath: string): boolean {
  const base = path.basename(filePath);
  const normalized = String(base || '').trim().toLowerCase();
  if (!normalized) return false;
  if (normalized.endsWith('~')) return true;
  const ext = path.extname(normalized);
  const stem = ext ? normalized.slice(0, -ext.length) : normalized;
  return GHOST_STEM_PATTERN.test(stem);
}

function buildMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Dashboard Segment Ghost File Guard');
  lines.push('');
  lines.push(`- Generated at: ${report.generated_at}`);
  lines.push(`- Revision: ${report.revision}`);
  lines.push(`- Strict mode: ${report.strict ? 'true' : 'false'}`);
  lines.push(`- Scanned segment files: ${report.scanned_segment_files}`);
  lines.push(`- Violations: ${report.violation_count}`);
  lines.push(`- Pass: ${report.ok ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Violations');
  lines.push('');
  lines.push('| Path | Reason |');
  lines.push('| --- | --- |');
  const sample = Array.isArray(report.violations) ? report.violations : [];
  for (const row of sample) {
    lines.push(`| ${String(row.path || '')} | ${String(row.reason || '')} |`);
  }
  if (!sample.length) lines.push('| (none) | - |');
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
  const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN_PATH, 400);
  const trackedFiles = listTrackedFiles();
  const segmentFiles = trackedFiles.filter(isSegmentPartPath);

  const violations = segmentFiles
    .filter(isGhostSegmentPartFile)
    .map((filePath) => ({
      path: filePath,
      reason: 'ghost_segment_part_file_forbidden',
    }));

  const report = {
    ok: violations.length === 0,
    type: 'dashboard_segment_ghost_file_guard',
    generated_at: new Date().toISOString(),
    strict: strictOut.strict,
    revision: currentRevision(),
    scanned_segment_files: segmentFiles.length,
    violation_count: violations.length,
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
