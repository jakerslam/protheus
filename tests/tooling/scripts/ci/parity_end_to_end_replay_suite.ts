#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/parity_end_to_end_replay_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/PARITY_END_TO_END_REPLAY_CURRENT.md';
const DEFAULT_OUT_ALIAS = 'artifacts/parity_end_to_end_replay_latest.json';

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
};

type StageSpec = {
  id: string;
  name: string;
  fixturePath: string;
  artifactPath: string;
};

type StageResult = {
  id: string;
  name: string;
  fixture_path: string;
  artifact_path: string;
  fixture_case_count: number;
  artifact_present: boolean;
  artifact_ok: boolean;
  ok: boolean;
  failure_reasons: string[];
};

function parseArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: false, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 500),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD, 500),
  };
}

function readJsonMaybe(filePath: string): any | null {
  try {
    return JSON.parse(fs.readFileSync(path.resolve(ROOT, filePath), 'utf8'));
  } catch {
    return null;
  }
}

function countFixtureCases(value: any): number {
  if (Array.isArray(value)) return value.length;
  if (!value || typeof value !== 'object') return 0;
  const preferredArrays = ['cases', 'scenarios', 'tests', 'matrix', 'rows']
    .map((key) => value[key])
    .filter((row) => Array.isArray(row)) as any[][];
  if (preferredArrays.length > 0) {
    return preferredArrays.reduce((sum, row) => sum + row.length, 0);
  }
  const nestedArrayCount = Object.values(value)
    .filter((row) => Array.isArray(row))
    .reduce((sum, row) => sum + (row as any[]).length, 0);
  if (nestedArrayCount > 0) return nestedArrayCount;
  return Object.keys(value).length;
}

function artifactPass(payload: any): boolean {
  if (!payload || typeof payload !== 'object') return false;
  if (typeof payload.ok === 'boolean') return payload.ok;
  if (typeof payload.pass === 'boolean') return payload.pass;
  if (payload.summary && typeof payload.summary.pass === 'boolean') return payload.summary.pass;
  if (payload.summary && typeof payload.summary.ok === 'boolean') return payload.summary.ok;
  return false;
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# PARITY END-TO-END REPLAY SUITE');
  lines.push('');
  lines.push(`- generated_at: ${payload.generated_at}`);
  lines.push(`- revision: ${payload.revision}`);
  lines.push(`- ok: ${payload.ok}`);
  lines.push(`- strict: ${payload.strict}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- stage_count: ${payload.summary.stage_count}`);
  lines.push(`- stage_pass_count: ${payload.summary.stage_pass_count}`);
  lines.push(`- stage_pass_rate: ${payload.summary.stage_pass_rate.toFixed(4)}`);
  lines.push('');
  lines.push('## Stage Trace');
  for (const row of payload.stage_trace || []) {
    lines.push(
      `- [${row.ok ? 'x' : ' '}] ${row.id} fixture_cases=${row.fixture_case_count} artifact_present=${row.artifact_present} artifact_ok=${row.artifact_ok}`,
    );
    for (const reason of row.failure_reasons || []) {
      lines.push(`  - reason: ${reason}`);
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[]): number {
  const args = parseArgs(argv);
  const stages: StageSpec[] = [
    {
      id: 'typed_probe_routing',
      name: 'typed probe + routing',
      fixturePath: 'tests/tooling/fixtures/tool_route_misdirection_matrix.json',
      artifactPath: 'core/local/artifacts/typed_probe_contract_matrix_guard_current.json',
    },
    {
      id: 'workspace_tooling',
      name: 'workspace tooling',
      fixturePath: 'tests/tooling/fixtures/workspace_path_targeting_replay_matrix.json',
      artifactPath: 'core/local/artifacts/workspace_tooling_reliability_current.json',
    },
    {
      id: 'web_tooling',
      name: 'web tooling extraction',
      fixturePath: 'tests/tooling/fixtures/web_tooling_extraction_contract_matrix.json',
      artifactPath: 'core/local/artifacts/web_tooling_reliability_current.json',
    },
    {
      id: 'synthesis',
      name: 'mixed-evidence synthesis',
      fixturePath: 'tests/tooling/fixtures/synthesis_mixed_evidence_regression_matrix.json',
      artifactPath: 'core/local/artifacts/synthesis_mixed_evidence_quality_current.json',
    },
    {
      id: 'recovery',
      name: 'workflow recovery',
      fixturePath: 'tests/tooling/fixtures/workflow_failure_recovery_matrix.json',
      artifactPath: 'core/local/artifacts/workflow_failure_recovery_current.json',
    },
  ];

  const stageTrace: StageResult[] = stages.map((stage) => {
    const fixturePayload = readJsonMaybe(stage.fixturePath);
    const artifactPayload = readJsonMaybe(stage.artifactPath);
    const fixtureCaseCount = countFixtureCases(fixturePayload);
    const artifactPresent = artifactPayload != null;
    const artifactOk = artifactPass(artifactPayload);
    const failureReasons: string[] = [];
    if (fixtureCaseCount <= 0) {
      failureReasons.push('fixture_cases_missing_or_empty');
    }
    if (!artifactPresent) {
      failureReasons.push('artifact_missing');
    } else if (!artifactOk) {
      failureReasons.push('artifact_not_ok');
    }
    return {
      id: stage.id,
      name: stage.name,
      fixture_path: stage.fixturePath,
      artifact_path: stage.artifactPath,
      fixture_case_count: fixtureCaseCount,
      artifact_present: artifactPresent,
      artifact_ok: artifactOk,
      ok: failureReasons.length === 0,
      failure_reasons: failureReasons,
    };
  });

  const stagePassCount = stageTrace.filter((row) => row.ok).length;
  const stageCount = stageTrace.length;
  const stagePassRate = stageCount > 0 ? stagePassCount / stageCount : 0;
  const ok = stagePassCount === stageCount;
  const payload = {
    ok,
    strict: args.strict,
    type: 'parity_end_to_end_replay',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
      stage_count: stageCount,
    },
    summary: {
      stage_count: stageCount,
      stage_pass_count: stagePassCount,
      stage_pass_rate: stagePassRate,
    },
    stage_trace: stageTrace,
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

