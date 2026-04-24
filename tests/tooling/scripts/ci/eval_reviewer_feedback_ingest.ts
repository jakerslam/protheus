#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

const DEFAULT_FEEDBACK_PATH = 'local/state/ops/eval_agent_chat_monitor/reviewer_feedback.jsonl';
const DEFAULT_OUT_PATH = 'core/local/artifacts/eval_reviewer_feedback_weekly_current.json';
const DEFAULT_OUT_LATEST_PATH = 'artifacts/eval_reviewer_feedback_weekly_latest.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/EVAL_REVIEWER_FEEDBACK_WEEKLY_CURRENT.md';
const DEFAULT_WINDOW_DAYS = 7;

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT_PATH });
  const windowDays = Number.parseInt(cleanText(readFlag(argv, 'window-days') || '', 20), 10);
  return {
    strict: common.strict,
    feedbackPath: cleanText(readFlag(argv, 'feedback') || DEFAULT_FEEDBACK_PATH, 500),
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT_PATH, 500),
    outLatestPath: cleanText(readFlag(argv, 'out-latest') || DEFAULT_OUT_LATEST_PATH, 500),
    markdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MARKDOWN_PATH, 500),
    windowDays: Number.isFinite(windowDays) && windowDays > 0 ? windowDays : DEFAULT_WINDOW_DAYS,
  };
}

function readJson(filePath: string): any | null {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function readJsonl(filePath: string): any[] {
  try {
    return fs
      .readFileSync(filePath, 'utf8')
      .split(/\r?\n/)
      .filter(Boolean)
      .map((line) => {
        try {
          return JSON.parse(line);
        } catch {
          return null;
        }
      })
      .filter(Boolean) as any[];
  } catch {
    return [];
  }
}

function parseIso(raw: string): number {
  const parsed = Date.parse(cleanText(raw, 120));
  return Number.isFinite(parsed) ? parsed : 0;
}

function scoreFromVerdict(raw: string): number {
  const verdict = cleanText(raw, 40).toLowerCase();
  if (verdict === 'correct') return 1;
  if (verdict === 'partial') return 0.5;
  return 0;
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Eval Reviewer Feedback Weekly Report (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report.generated_at || '', 120)}`);
  lines.push(`- ok: ${report.ok === true ? 'true' : 'false'}`);
  lines.push(`- window_days: ${Number(report.summary?.window_days || 0)}`);
  lines.push(`- feedback_rows: ${Number(report.summary?.feedback_rows || 0)}`);
  lines.push(`- correctness_score: ${Number(report.summary?.correctness_score || 0).toFixed(3)}`);
  lines.push(`- calibration_delta_vs_previous: ${Number(report.summary?.calibration_delta_vs_previous || 0).toFixed(3)}`);
  lines.push('');
  lines.push('## Verdict counts');
  const counts = report.summary?.verdict_counts || {};
  lines.push(`- correct: ${Number(counts.correct || 0)}`);
  lines.push(`- partial: ${Number(counts.partial || 0)}`);
  lines.push(`- incorrect: ${Number(counts.incorrect || 0)}`);
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const root = process.cwd();
  const feedbackAbs = path.resolve(root, args.feedbackPath);
  const outLatestAbs = path.resolve(root, args.outLatestPath);
  const markdownAbs = path.resolve(root, args.markdownPath);
  const nowMs = Date.now();
  const nowIso = new Date(nowMs).toISOString();
  const windowStartMs = nowMs - args.windowDays * 24 * 60 * 60 * 1000;

  const rows = readJsonl(feedbackAbs);
  const weeklyRows = rows.filter((row) => parseIso(row?.ts || row?.timestamp || '') >= windowStartMs);
  const previous = readJson(outLatestAbs) || {};

  const verdictCounts = {
    correct: 0,
    partial: 0,
    incorrect: 0,
  };
  for (const row of weeklyRows) {
    const verdict = cleanText(row?.verdict || '', 40).toLowerCase();
    if (verdict === 'correct') verdictCounts.correct += 1;
    else if (verdict === 'partial') verdictCounts.partial += 1;
    else verdictCounts.incorrect += 1;
  }

  const total = weeklyRows.length;
  const correctnessScore = total > 0
    ? weeklyRows.reduce((acc, row) => acc + scoreFromVerdict(row?.verdict || ''), 0) / total
    : 0;
  const previousScore = Number(previous?.summary?.correctness_score || 0);
  const calibrationDelta = Number((correctnessScore - previousScore).toFixed(3));
  const malformedRows = rows.filter((row) => !row || typeof row !== 'object').length;

  const checks = [
    {
      id: 'feedback_ingestion_contract',
      ok: malformedRows === 0,
      detail: `rows=${rows.length};malformed=${malformedRows}`,
    },
    {
      id: 'weekly_window_contract',
      ok: true,
      detail: `window_days=${args.windowDays};rows_in_window=${weeklyRows.length}`,
    },
    {
      id: 'calibration_delta_contract',
      ok: true,
      detail: `current=${correctnessScore.toFixed(3)};previous=${previousScore.toFixed(3)};delta=${calibrationDelta.toFixed(3)}`,
    },
  ];

  const report = {
    type: 'eval_reviewer_feedback_weekly_report',
    schema_version: 1,
    generated_at: nowIso,
    ok: checks.every((row) => row.ok),
    checks,
    summary: {
      window_days: args.windowDays,
      feedback_rows: weeklyRows.length,
      correctness_score: Number(correctnessScore.toFixed(3)),
      calibration_delta_vs_previous: calibrationDelta,
      verdict_counts: verdictCounts,
      status: weeklyRows.length > 0 ? 'active' : 'awaiting_feedback',
    },
    feedback_rows: weeklyRows.slice(0, 200).map((row) => ({
      ts: cleanText(row?.ts || row?.timestamp || '', 120),
      issue_id: cleanText(row?.issue_id || '', 120) || null,
      verdict: cleanText(row?.verdict || '', 40).toLowerCase(),
      note: cleanText(row?.note || '', 240) || null,
    })),
    sources: {
      feedback: args.feedbackPath,
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

