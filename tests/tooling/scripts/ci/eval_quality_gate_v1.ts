#!/usr/bin/env tsx

import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';

const DEFAULT_QUALITY_PATH = 'artifacts/eval_quality_metrics_latest.json';
const DEFAULT_MONITOR_PATH = 'local/state/ops/eval_agent_chat_monitor/latest.json';
const DEFAULT_JUDGE_HUMAN_PATH = 'artifacts/eval_judge_human_agreement_latest.json';
const DEFAULT_HISTORY_PATH = 'local/state/ops/eval_quality_gate_v1/history.json';
const DEFAULT_OUT_PATH = 'core/local/artifacts/eval_quality_gate_v1_current.json';
const DEFAULT_OUT_LATEST_PATH = 'artifacts/eval_quality_gate_v1_latest.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/EVAL_QUALITY_GATE_V1_CURRENT.md';
const DEFAULT_REQUIRED_CONSECUTIVE_PASSES = 3;

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT_PATH });
  const requiredConsecutivePasses = Number.parseInt(
    cleanText(readFlag(argv, 'required-consecutive-passes') || '', 20),
    10,
  );
  return {
    strict: common.strict,
    qualityPath: cleanText(readFlag(argv, 'quality') || DEFAULT_QUALITY_PATH, 500),
    monitorPath: cleanText(readFlag(argv, 'monitor') || DEFAULT_MONITOR_PATH, 500),
    judgeHumanPath: cleanText(readFlag(argv, 'judge-human') || DEFAULT_JUDGE_HUMAN_PATH, 500),
    historyPath: cleanText(readFlag(argv, 'history') || DEFAULT_HISTORY_PATH, 500),
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT_PATH, 500),
    outLatestPath: cleanText(readFlag(argv, 'out-latest') || DEFAULT_OUT_LATEST_PATH, 500),
    markdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MARKDOWN_PATH, 500),
    requiredConsecutivePasses:
      Number.isFinite(requiredConsecutivePasses) && requiredConsecutivePasses > 0
        ? requiredConsecutivePasses
        : DEFAULT_REQUIRED_CONSECUTIVE_PASSES,
  };
}

function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const root = process.cwd();
  const commandArgs = [
    'run',
    '--quiet',
    '--manifest-path',
    'surface/orchestration/Cargo.toml',
    '--bin',
    'eval-runtime',
    '--',
    'quality-gate',
    `--strict=${args.strict ? 1 : 0}`,
    `--quality=${args.qualityPath}`,
    `--monitor=${args.monitorPath}`,
    `--judge-human=${args.judgeHumanPath}`,
    `--history=${args.historyPath}`,
    `--out=${args.outPath}`,
    `--out-latest=${args.outLatestPath}`,
    `--out-markdown=${args.markdownPath}`,
    `--required-consecutive-passes=${args.requiredConsecutivePasses}`,
  ];
  const proc = spawnSync('cargo', commandArgs, {
    cwd: root,
    stdio: 'inherit',
    env: process.env,
  });
  if (typeof proc.status === 'number') return proc.status;
  if (proc.error) {
    console.error(proc.error.message);
  }
  return 2;
}

process.exit(run(process.argv.slice(2)));
