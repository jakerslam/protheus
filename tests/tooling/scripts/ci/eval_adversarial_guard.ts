#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

const DEFAULT_MATRIX_PATH = 'tests/tooling/fixtures/eval_adversarial_matrix.json';
const DEFAULT_OUT_PATH = 'core/local/artifacts/eval_adversarial_guard_current.json';
const DEFAULT_OUT_LATEST_PATH = 'artifacts/eval_adversarial_guard_latest.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/EVAL_ADVERSARIAL_GUARD_CURRENT.md';

const WORKFLOW_RETRY_PATTERN = /final workflow state was unexpected|final reply did not render|please retry so i can rerun the chain cleanly/i;
const WEB_CARD_PATTERN = /title:|excerpt:|originalurl:|featuredcontent:|provider:/i;
const FILE_INTENT_PATTERN = /file tool|file tooling|local file|workspace|directory|local dir/i;

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT_PATH });
  return {
    strict: common.strict,
    matrixPath: cleanText(readFlag(argv, 'matrix') || DEFAULT_MATRIX_PATH, 500),
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT_PATH, 500),
    outLatestPath: cleanText(readFlag(argv, 'out-latest') || DEFAULT_OUT_LATEST_PATH, 500),
    markdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MARKDOWN_PATH, 500),
  };
}

function readJson(filePath: string): any | null {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function isQuotedGuard(assistantLower: string): boolean {
  return /you said|you wrote|quote|quoted|as you said|as you wrote/.test(assistantLower) || /^>\s/m.test(assistantLower);
}

function detectIssue(userLower: string, assistantLower: string): boolean {
  const workflowRetry = WORKFLOW_RETRY_PATTERN.test(assistantLower);
  const webCard = WEB_CARD_PATTERN.test(assistantLower);
  const fileIntent = FILE_INTENT_PATTERN.test(userLower);
  return workflowRetry || (fileIntent && webCard);
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Eval Adversarial Guard (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report.generated_at || '', 120)}`);
  lines.push(`- ok: ${report.ok === true ? 'true' : 'false'}`);
  lines.push(`- case_count: ${Number(report.summary?.case_count || 0)}`);
  lines.push(`- mismatch_count: ${Number(report.summary?.mismatch_count || 0)}`);
  lines.push('');
  const mismatches = Array.isArray(report.mismatches) ? report.mismatches : [];
  lines.push('## Mismatches');
  if (mismatches.length === 0) {
    lines.push('- none');
  } else {
    mismatches.forEach((row) => {
      lines.push(`- ${cleanText(row?.id || 'case', 120)} expected_issue=${Boolean(row?.expected_issue_detected)} actual_issue=${Boolean(row?.actual_issue_detected)} expected_quote=${Boolean(row?.expected_quoted_guard)} actual_quote=${Boolean(row?.actual_quoted_guard)}`);
    });
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const root = process.cwd();
  const matrixAbs = path.resolve(root, args.matrixPath);
  const outLatestAbs = path.resolve(root, args.outLatestPath);
  const markdownAbs = path.resolve(root, args.markdownPath);
  const nowIso = new Date().toISOString();

  const matrix = readJson(matrixAbs) || {};
  const cases = Array.isArray(matrix?.cases) ? matrix.cases : [];
  const results = cases.map((row: any) => {
    const userLower = cleanText(row?.user_text || '', 4000).toLowerCase();
    const assistantLower = cleanText(row?.assistant_text || '', 4000).toLowerCase();
    const actualQuoted = isQuotedGuard(assistantLower);
    const actualIssue = detectIssue(userLower, assistantLower) && !actualQuoted;
    return {
      id: cleanText(row?.id || 'case', 120),
      expected_quoted_guard: Boolean(row?.expected?.quoted_guard_expected),
      expected_issue_detected: Boolean(row?.expected?.issue_detected_expected),
      actual_quoted_guard: actualQuoted,
      actual_issue_detected: actualIssue,
    };
  });
  const mismatches = results.filter((row) => {
    return row.expected_quoted_guard !== row.actual_quoted_guard
      || row.expected_issue_detected !== row.actual_issue_detected;
  });

  const checks = [
    { id: 'matrix_present', ok: fs.existsSync(matrixAbs), detail: args.matrixPath },
    {
      id: 'adversarial_case_contract',
      ok: cases.length > 0,
      detail: `cases=${cases.length}`,
    },
    {
      id: 'adversarial_escape_contract',
      ok: mismatches.length === 0,
      detail: `mismatches=${mismatches.length}`,
    },
  ];

  const report = {
    type: 'eval_adversarial_guard',
    schema_version: 1,
    generated_at: nowIso,
    ok: checks.every((row) => row.ok),
    checks,
    summary: {
      case_count: cases.length,
      mismatch_count: mismatches.length,
    },
    results,
    mismatches,
    sources: {
      matrix: args.matrixPath,
    },
  };

  writeJsonArtifact(outLatestAbs, report);
  writeTextArtifact(markdownAbs, renderMarkdown(report));
  return emitStructuredResult(report, {
    outPath: path.resolve(root, args.outPath),
    strict: args.strict,
    ok: report.ok,
  });
}

process.exit(run(process.argv.slice(2)));

