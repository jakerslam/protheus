#!/usr/bin/env node
/* eslint-disable no-console */
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/orchestration_adapter_fallback_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/ORCHESTRATION_ADAPTER_FALLBACK_GUARD_CURRENT.md';
const TEST_NAME = 'non_legacy_surface_fixture_fallback_rate_stays_below_threshold';

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

function thresholdFromSource(): number | null {
  const sourcePath = path.resolve(ROOT, 'surface/orchestration/tests/conformance.rs');
  const source = fs.readFileSync(sourcePath, 'utf8');
  const marker = new RegExp(`${TEST_NAME}[\\s\\S]*?fallback_rate\\s*<=\\s*([0-9.]+)`, 'm');
  const match = source.match(marker);
  return match ? Number(match[1]) : null;
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Orchestration Adapter Fallback Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Threshold: ${payload.threshold ?? 'unknown'}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Command');
  lines.push(`- ${payload.command.join(' ')}`);
  lines.push('');
  lines.push('## Output');
  lines.push('```text');
  lines.push(String(payload.stdout || payload.stderr || '').trim().slice(0, 4000));
  lines.push('```');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[]): number {
  const args = resolveArgs(argv);
  const threshold = thresholdFromSource();
  const command = [
    'cargo',
    'test',
    '--manifest-path',
    'surface/orchestration/Cargo.toml',
    TEST_NAME,
    '--',
    '--exact',
  ];
  const result = spawnSync(command[0], command.slice(1), {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  const ok = result.status === 0;
  const payload = {
    ok,
    type: 'orchestration_adapter_fallback_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
    },
    test_name: TEST_NAME,
    threshold,
    command,
    summary: {
      pass: ok,
      exit_code: result.status ?? 1,
      signal: result.signal ?? null,
    },
    stdout: String(result.stdout || ''),
    stderr: String(result.stderr || ''),
  };

  writeTextArtifact(args.outMarkdown, toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok,
  });
}

process.exit(run(process.argv.slice(2)));
