#!/usr/bin/env tsx

import { spawnSync } from 'node:child_process';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';

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

function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const proc = spawnSync('cargo', [
    'run',
    '--quiet',
    '--manifest-path',
    'surface/orchestration/Cargo.toml',
    '--bin',
    'eval_runtime',
    '--',
    'reviewer-feedback',
    `--strict=${args.strict ? 1 : 0}`,
    `--feedback=${args.feedbackPath}`,
    `--out=${args.outPath}`,
    `--out-latest=${args.outLatestPath}`,
    `--out-markdown=${args.markdownPath}`,
    `--window-days=${args.windowDays}`,
  ], {
    cwd: process.cwd(),
    stdio: 'inherit',
    env: process.env,
  });
  if (typeof proc.status === 'number') return proc.status;
  if (proc.error) console.error(proc.error.message);
  return 2;
}

process.exit(run(process.argv.slice(2)));
