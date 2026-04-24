#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/parity_release_gate_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/PARITY_RELEASE_GATE_CURRENT.md';
const DEFAULT_OUT_ALIAS = 'artifacts/parity_release_gate_latest.json';
const DEFAULT_E2E_ARTIFACT = 'core/local/artifacts/parity_end_to_end_replay_current.json';
const DEFAULT_TREND_ARTIFACT = 'core/local/artifacts/parity_trend_current.json';

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  e2eArtifact: string;
  trendArtifact: string;
  minE2eStagePassRate: number;
  minTrendOverallScore: number;
  minTrendConfidenceLow: number;
};

type CheckRow = {
  id: string;
  ok: boolean;
  detail: string;
};

function clamp01(value: number): number {
  if (!Number.isFinite(value)) return 0;
  if (value < 0) return 0;
  if (value > 1) return 1;
  return value;
}

function toNum(value: unknown): number | null {
  if (typeof value === 'number' && Number.isFinite(value)) return value;
  if (typeof value === 'string' && value.trim().length > 0) {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) return parsed;
  }
  return null;
}

function boolLike(value: any): boolean {
  if (!value || typeof value !== 'object') return false;
  if (typeof value.ok === 'boolean') return value.ok;
  if (typeof value.pass === 'boolean') return value.pass;
  if (value.summary && typeof value.summary.pass === 'boolean') return value.summary.pass;
  if (value.summary && typeof value.summary.ok === 'boolean') return value.summary.ok;
  return false;
}

function readJsonMaybe(filePath: string): any | null {
  try {
    return JSON.parse(fs.readFileSync(path.resolve(ROOT, filePath), 'utf8'));
  } catch {
    return null;
  }
}

function parseThreshold(flagValue: string | undefined, envValue: string | undefined, fallback: number): number {
  const fromFlag = toNum(flagValue);
  if (fromFlag != null) return clamp01(fromFlag);
  const fromEnv = toNum(envValue);
  if (fromEnv != null) return clamp01(fromEnv);
  return fallback;
}

function parseArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: false, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 500),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD, 500),
    e2eArtifact: cleanText(readFlag(argv, 'parity-e2e-artifact') || DEFAULT_E2E_ARTIFACT, 500),
    trendArtifact: cleanText(readFlag(argv, 'parity-trend-artifact') || DEFAULT_TREND_ARTIFACT, 500),
    minE2eStagePassRate: parseThreshold(
      readFlag(argv, 'min-e2e-stage-pass-rate'),
      process.env.INFRING_PARITY_RELEASE_GATE_MIN_E2E_STAGE_PASS_RATE,
      1,
    ),
    minTrendOverallScore: parseThreshold(
      readFlag(argv, 'min-trend-overall-score'),
      process.env.INFRING_PARITY_RELEASE_GATE_MIN_TREND_OVERALL_SCORE,
      0.9,
    ),
    minTrendConfidenceLow: parseThreshold(
      readFlag(argv, 'min-trend-confidence-low'),
      process.env.INFRING_PARITY_RELEASE_GATE_MIN_TREND_CONFIDENCE_LOW,
      0.6,
    ),
  };
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# PARITY RELEASE GATE CURRENT');
  lines.push('');
  lines.push(`- generated_at: ${payload.generated_at}`);
  lines.push(`- revision: ${payload.revision}`);
  lines.push(`- ok: ${payload.ok}`);
  lines.push(`- strict: ${payload.strict}`);
  lines.push('');
  lines.push('## Thresholds');
  lines.push(`- min_e2e_stage_pass_rate: ${payload.thresholds.min_e2e_stage_pass_rate.toFixed(4)}`);
  lines.push(`- min_trend_overall_score: ${payload.thresholds.min_trend_overall_score.toFixed(4)}`);
  lines.push(`- min_trend_confidence_low: ${payload.thresholds.min_trend_confidence_low.toFixed(4)}`);
  lines.push('');
  lines.push('## Checks');
  for (const row of payload.checks || []) {
    lines.push(`- [${row.ok ? 'x' : ' '}] ${row.id}: ${row.detail}`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[]): number {
  const args = parseArgs(argv);
  const e2e = readJsonMaybe(args.e2eArtifact);
  const trend = readJsonMaybe(args.trendArtifact);

  const e2eStagePassRate = toNum(e2e?.summary?.stage_pass_rate) ?? 0;
  const trendOverallScore = toNum(trend?.summary?.overall_score) ?? 0;
  const trendConfidenceLow = toNum(trend?.summary?.confidence_min) ?? 0;

  const checks: CheckRow[] = [];
  checks.push({
    id: 'parity_end_to_end_artifact_present',
    ok: e2e != null,
    detail: `path=${args.e2eArtifact}`,
  });
  checks.push({
    id: 'parity_trend_artifact_present',
    ok: trend != null,
    detail: `path=${args.trendArtifact}`,
  });
  checks.push({
    id: 'parity_end_to_end_ok',
    ok: boolLike(e2e),
    detail: `ok=${boolLike(e2e)} stage_pass_rate=${e2eStagePassRate.toFixed(4)}`,
  });
  checks.push({
    id: 'parity_trend_ok',
    ok: boolLike(trend),
    detail: `ok=${boolLike(trend)} overall_score=${trendOverallScore.toFixed(4)}`,
  });
  checks.push({
    id: 'parity_end_to_end_stage_pass_rate_threshold',
    ok: e2eStagePassRate >= args.minE2eStagePassRate,
    detail: `actual=${e2eStagePassRate.toFixed(4)} min=${args.minE2eStagePassRate.toFixed(4)}`,
  });
  checks.push({
    id: 'parity_trend_overall_score_threshold',
    ok: trendOverallScore >= args.minTrendOverallScore,
    detail: `actual=${trendOverallScore.toFixed(4)} min=${args.minTrendOverallScore.toFixed(4)}`,
  });
  checks.push({
    id: 'parity_trend_confidence_low_threshold',
    ok: trendConfidenceLow >= args.minTrendConfidenceLow,
    detail: `actual=${trendConfidenceLow.toFixed(4)} min=${args.minTrendConfidenceLow.toFixed(4)}`,
  });

  const failed = checks.filter((row) => !row.ok);
  const ok = failed.length === 0;

  const payload = {
    ok,
    strict: args.strict,
    type: 'parity_release_gate',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    thresholds: {
      min_e2e_stage_pass_rate: args.minE2eStagePassRate,
      min_trend_overall_score: args.minTrendOverallScore,
      min_trend_confidence_low: args.minTrendConfidenceLow,
    },
    artifacts: {
      parity_end_to_end_replay: {
        path: args.e2eArtifact,
        present: e2e != null,
      },
      parity_trend: {
        path: args.trendArtifact,
        present: trend != null,
      },
    },
    checks,
    failed_ids: failed.map((row) => row.id),
  };

  writeJsonArtifact(DEFAULT_OUT_ALIAS, payload);
  writeTextArtifact(args.outMarkdown, toMarkdown(payload));

  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok,
  });
}

process.exit(run(process.argv.slice(2)));
