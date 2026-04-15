#!/usr/bin/env node
/* eslint-disable no-console */
import { spawnSync } from 'node:child_process';
import { parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/orchestration_planner_quality_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/ORCHESTRATION_PLANNER_QUALITY_GUARD_CURRENT.md';
const TEST_NAME = 'planner_quality_fixture_metrics_stay_within_thresholds';

type ScriptArgs = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
};

function resolveArgs(argv: string[]): ScriptArgs {
  return {
    strict: argv.includes('--strict') || parseBool(readFlag(argv, 'strict'), false),
    outJson: readFlag(argv, 'out-json') || DEFAULT_OUT_JSON,
    outMarkdown: readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD,
  };
}

function parsePlannerMetrics(output: string): unknown | null {
  const marker = output.match(/planner_quality_metrics=(\{.*\})/m);
  if (!marker) {
    return null;
  }
  try {
    return JSON.parse(marker[1]);
  } catch {
    return null;
  }
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Orchestration Planner Quality Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Command');
  lines.push(`- ${payload.command.join(' ')}`);
  if (payload.metrics) {
    lines.push('');
    lines.push('## Planner Metrics');
    lines.push('```json');
    lines.push(JSON.stringify(payload.metrics, null, 2));
    lines.push('```');
  }
  lines.push('');
  lines.push('## Output');
  lines.push('```text');
  lines.push(String(payload.output_excerpt || '').trim().slice(0, 6000));
  lines.push('```');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[]): number {
  const args = resolveArgs(argv);
  const command = [
    'cargo',
    'test',
    '--manifest-path',
    'surface/orchestration/Cargo.toml',
    TEST_NAME,
    '--',
    '--exact',
    '--nocapture',
  ];
  const result = spawnSync(command[0], command.slice(1), {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  const ok = result.status === 0;
  const output = [String(result.stdout || ''), String(result.stderr || '')]
    .filter(Boolean)
    .join('\n')
    .trim();
  const payload = {
    ok,
    type: 'orchestration_planner_quality_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
    },
    test_name: TEST_NAME,
    command,
    summary: {
      pass: ok,
      exit_code: result.status ?? 1,
      signal: result.signal ?? null,
    },
    metrics: parsePlannerMetrics(output),
    output_excerpt: output,
  };

  writeTextArtifact(args.outMarkdown, toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok,
  });
}

process.exit(run(process.argv.slice(2)));
