#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

const DEFAULT_OUT_PATH = 'core/local/artifacts/eval_runtime_authority_guard_current.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/EVAL_RUNTIME_AUTHORITY_GUARD_CURRENT.md';

const RUNTIME_AUTHORITY_FILES = [
  'surface/orchestration/src/eval.rs',
  'surface/orchestration/src/bin/eval_runtime.rs',
];

const WRAPPER_FILES = [
  'tests/tooling/scripts/ci/eval_quality_gate_v1.ts',
  'tests/tooling/scripts/ci/eval_judge_human_agreement_guard.ts',
];

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT_PATH });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT_PATH, 500),
    markdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MARKDOWN_PATH, 500),
  };
}

function readText(filePath: string): string {
  try {
    return fs.readFileSync(filePath, 'utf8');
  } catch {
    return '';
  }
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Eval Runtime Authority Guard (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report.generated_at || '', 120)}`);
  lines.push(`- ok: ${report.ok === true ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Checks');
  for (const check of Array.isArray(report.checks) ? report.checks : []) {
    lines.push(
      `- ${check.ok ? 'PASS' : 'FAIL'} \`${cleanText(check.id || 'unknown', 120)}\` — ${cleanText(check.detail || '', 240)}`,
    );
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const root = process.cwd();
  const nowIso = new Date().toISOString();

  const checks: Array<{ id: string; ok: boolean; detail: string }> = [];

  for (const rel of RUNTIME_AUTHORITY_FILES) {
    const abs = path.resolve(root, rel);
    checks.push({
      id: `runtime_file_present:${rel}`,
      ok: fs.existsSync(abs),
      detail: rel,
    });
  }

  const runtimeEvalSource = readText(path.resolve(root, 'surface/orchestration/src/eval.rs'));
  checks.push({
    id: 'runtime_exports_quality_gate_eval_contract',
    ok: runtimeEvalSource.includes('pub fn evaluate_quality_gate('),
    detail: 'surface/orchestration/src/eval.rs::evaluate_quality_gate',
  });
  checks.push({
    id: 'runtime_exports_judge_human_eval_contract',
    ok: runtimeEvalSource.includes('pub fn evaluate_judge_human_agreement('),
    detail: 'surface/orchestration/src/eval.rs::evaluate_judge_human_agreement',
  });

  for (const rel of WRAPPER_FILES) {
    const source = readText(path.resolve(root, rel));
    checks.push({
      id: `wrapper_delegates_to_runtime_bin:${rel}`,
      ok: source.includes('--bin') && source.includes('eval_runtime'),
      detail: rel,
    });
    checks.push({
      id: `wrapper_no_authority_logic:${rel}`,
      ok:
        !source.includes('thresholdViolations')
        && !source.includes('regressionViolations')
        && !source.includes('writeJsonArtifact('),
      detail: rel,
    });
  }

  const report = {
    type: 'eval_runtime_authority_guard',
    schema_version: 1,
    generated_at: nowIso,
    ok: checks.every((row) => row.ok),
    checks,
    summary: {
      runtime_authority_files: RUNTIME_AUTHORITY_FILES,
      wrapper_files: WRAPPER_FILES,
      failing_checks: checks.filter((row) => !row.ok).map((row) => row.id),
    },
  };

  writeTextArtifact(path.resolve(root, args.markdownPath), renderMarkdown(report));
  return emitStructuredResult(report, {
    outPath: path.resolve(root, args.outPath),
    strict: args.strict,
    ok: report.ok,
  });
}

process.exit(run(process.argv.slice(2)));
