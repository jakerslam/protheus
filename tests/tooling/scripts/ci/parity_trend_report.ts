#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/parity_trend_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/PARITY_TREND_CURRENT.md';
const DEFAULT_OUT_ALIAS = 'artifacts/parity_trend_latest.json';
const DEFAULT_HISTORY = 'local/state/ops/parity/parity_trend_history.jsonl';

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  historyPath: string;
};

type GapSpec = {
  id: string;
  name: string;
  artifactPath: string;
};

type GapRow = {
  id: string;
  name: string;
  artifact_path: string;
  artifact_present: boolean;
  artifact_ok: boolean;
  sample_count: number;
  current_score: number;
  previous_score: number | null;
  score_delta: number;
  confidence_low: number;
  confidence_high: number;
};

type HistorySnapshot = {
  generated_at: string;
  revision: string;
  overall_score: number;
  gap_scores: Record<string, number>;
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

function parseArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: false, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 500),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD, 500),
    historyPath: cleanText(readFlag(argv, 'history') || DEFAULT_HISTORY, 500),
  };
}

function readJsonMaybe(filePath: string): any | null {
  try {
    return JSON.parse(fs.readFileSync(path.resolve(ROOT, filePath), 'utf8'));
  } catch {
    return null;
  }
}

function artifactBool(payload: any): boolean {
  if (!payload || typeof payload !== 'object') return false;
  if (typeof payload.ok === 'boolean') return payload.ok;
  if (typeof payload.pass === 'boolean') return payload.pass;
  if (payload.summary && typeof payload.summary.pass === 'boolean') return payload.summary.pass;
  if (payload.summary && typeof payload.summary.ok === 'boolean') return payload.summary.ok;
  return false;
}

function extractScore(payload: any): number {
  if (!payload || typeof payload !== 'object') return 0;
  const scoreCandidates = [
    payload?.summary?.score,
    payload?.summary?.pass_rate,
    payload?.summary?.stage_pass_rate,
    payload?.summary?.success_rate,
    payload?.summary?.evidence_coverage_ratio,
    payload?.metrics?.score,
  ];
  for (const row of scoreCandidates) {
    const value = toNum(row);
    if (value != null) return clamp01(value);
  }
  return artifactBool(payload) ? 1 : 0;
}

function extractSampleCount(payload: any): number {
  if (!payload || typeof payload !== 'object') return 0;
  const countCandidates = [
    payload?.summary?.sample_count,
    payload?.summary?.sample_points,
    payload?.summary?.case_count,
    payload?.summary?.stage_count,
    payload?.summary?.scenario_count,
    payload?.summary?.artifact_count,
  ];
  for (const row of countCandidates) {
    const value = toNum(row);
    if (value != null && value >= 0) return Math.floor(value);
  }
  if (Array.isArray(payload?.stage_trace)) return payload.stage_trace.length;
  if (Array.isArray(payload?.cases)) return payload.cases.length;
  if (Array.isArray(payload?.scenarios)) return payload.scenarios.length;
  return 0;
}

function confidenceBand(score: number, sampleCount: number): { low: number; high: number } {
  const n = Math.max(1, sampleCount);
  const halfWidth = Math.min(0.35, 1 / Math.sqrt(n + 1));
  return {
    low: clamp01(score - halfWidth),
    high: clamp01(score + halfWidth),
  };
}

function readHistory(historyPath: string): HistorySnapshot[] {
  const resolved = path.resolve(ROOT, historyPath);
  if (!fs.existsSync(resolved)) return [];
  const lines = fs
    .readFileSync(resolved, 'utf8')
    .split(/\r?\n/)
    .map((row) => row.trim())
    .filter((row) => row.length > 0);
  const snapshots: HistorySnapshot[] = [];
  for (const line of lines) {
    try {
      const parsed = JSON.parse(line);
      if (
        parsed &&
        typeof parsed === 'object' &&
        typeof parsed.generated_at === 'string' &&
        typeof parsed.revision === 'string' &&
        typeof parsed.overall_score === 'number' &&
        parsed.gap_scores &&
        typeof parsed.gap_scores === 'object'
      ) {
        snapshots.push(parsed as HistorySnapshot);
      }
    } catch {
      // ignore malformed history lines
    }
  }
  return snapshots;
}

function appendHistory(historyPath: string, snapshot: HistorySnapshot): void {
  const resolved = path.resolve(ROOT, historyPath);
  fs.mkdirSync(path.dirname(resolved), { recursive: true });
  fs.appendFileSync(resolved, `${JSON.stringify(snapshot)}\n`);
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# PARITY TREND CURRENT');
  lines.push('');
  lines.push(`- generated_at: ${payload.generated_at}`);
  lines.push(`- revision: ${payload.revision}`);
  lines.push(`- ok: ${payload.ok}`);
  lines.push(`- strict: ${payload.strict}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- gap_count: ${payload.summary.gap_count}`);
  lines.push(`- gap_present_count: ${payload.summary.gap_present_count}`);
  lines.push(`- overall_score: ${payload.summary.overall_score.toFixed(4)}`);
  lines.push(`- overall_delta: ${payload.summary.overall_delta.toFixed(4)}`);
  lines.push(`- confidence_min: ${payload.summary.confidence_min.toFixed(4)}`);
  lines.push(`- confidence_max: ${payload.summary.confidence_max.toFixed(4)}`);
  lines.push('');
  lines.push('## Per-gap Scores');
  for (const row of payload.gaps || []) {
    const previous = row.previous_score == null ? 'n/a' : Number(row.previous_score).toFixed(4);
    lines.push(
      `- ${row.id}: score=${Number(row.current_score).toFixed(4)} prev=${previous} delta=${Number(row.score_delta).toFixed(4)} conf=[${Number(row.confidence_low).toFixed(4)}, ${Number(row.confidence_high).toFixed(4)}] present=${row.artifact_present} ok=${row.artifact_ok}`,
    );
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[]): number {
  const args = parseArgs(argv);
  const gaps: GapSpec[] = [
    {
      id: 'typed_probe_routing',
      name: 'Typed probe + routing',
      artifactPath: 'core/local/artifacts/typed_probe_contract_matrix_guard_current.json',
    },
    {
      id: 'workspace_tooling',
      name: 'Workspace tooling',
      artifactPath: 'core/local/artifacts/workspace_tooling_reliability_current.json',
    },
    {
      id: 'web_tooling',
      name: 'Web tooling extraction',
      artifactPath: 'core/local/artifacts/web_tooling_reliability_current.json',
    },
    {
      id: 'synthesis',
      name: 'Synthesis quality',
      artifactPath: 'core/local/artifacts/synthesis_mixed_evidence_quality_current.json',
    },
    {
      id: 'recovery',
      name: 'Workflow recovery',
      artifactPath: 'core/local/artifacts/workflow_failure_recovery_current.json',
    },
    {
      id: 'parity_end_to_end',
      name: 'Parity end-to-end replay',
      artifactPath: 'core/local/artifacts/parity_end_to_end_replay_current.json',
    },
  ];

  const history = readHistory(args.historyPath);
  const previous = history.length > 0 ? history[history.length - 1] : null;

  const rows: GapRow[] = gaps.map((gap) => {
    const payload = readJsonMaybe(gap.artifactPath);
    const artifactPresent = payload != null;
    const artifactOk = artifactBool(payload);
    const currentScore = artifactPresent ? extractScore(payload) : 0;
    const previousScore = previous?.gap_scores?.[gap.id] ?? null;
    const scoreDelta = previousScore == null ? 0 : currentScore - previousScore;
    const sampleCount = artifactPresent ? extractSampleCount(payload) : 0;
    const band = confidenceBand(currentScore, sampleCount);
    return {
      id: gap.id,
      name: gap.name,
      artifact_path: gap.artifactPath,
      artifact_present: artifactPresent,
      artifact_ok: artifactOk,
      sample_count: sampleCount,
      current_score: currentScore,
      previous_score: previousScore,
      score_delta: scoreDelta,
      confidence_low: band.low,
      confidence_high: band.high,
    };
  });

  const gapCount = rows.length;
  const gapPresentCount = rows.filter((row) => row.artifact_present).length;
  const overallScore = gapCount > 0 ? rows.reduce((sum, row) => sum + row.current_score, 0) / gapCount : 0;
  const overallDelta = gapCount > 0 ? rows.reduce((sum, row) => sum + row.score_delta, 0) / gapCount : 0;
  const confidenceMin = rows.length > 0 ? Math.min(...rows.map((row) => row.confidence_low)) : 0;
  const confidenceMax = rows.length > 0 ? Math.max(...rows.map((row) => row.confidence_high)) : 0;
  const ok = rows.every((row) => row.artifact_present && row.artifact_ok);

  const payload = {
    ok,
    strict: args.strict,
    type: 'parity_trend_report',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
      history_path: args.historyPath,
      gap_count: gapCount,
    },
    summary: {
      gap_count: gapCount,
      gap_present_count: gapPresentCount,
      overall_score: overallScore,
      overall_delta: overallDelta,
      confidence_min: confidenceMin,
      confidence_max: confidenceMax,
    },
    gaps: rows,
  };

  const historySnapshot: HistorySnapshot = {
    generated_at: payload.generated_at,
    revision: payload.revision,
    overall_score: overallScore,
    gap_scores: rows.reduce<Record<string, number>>((acc, row) => {
      acc[row.id] = row.current_score;
      return acc;
    }, {}),
  };
  appendHistory(args.historyPath, historySnapshot);

  writeJsonArtifact(DEFAULT_OUT_ALIAS, payload);
  writeTextArtifact(args.outMarkdown, toMarkdown(payload));

  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok,
  });
}

process.exit(run(process.argv.slice(2)));
