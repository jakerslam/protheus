#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/ci_workflow_rationalization_contract_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/CI_WORKFLOW_RATIONALIZATION_CONTRACT_CURRENT.md';

type ScriptArgs = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
};

type CheckRow = {
  id: string;
  ok: boolean;
  detail: string;
};

function resolveArgs(argv: string[]): ScriptArgs {
  return {
    strict: argv.includes('--strict') || parseBool(readFlag(argv, 'strict'), false),
    outJson: readFlag(argv, 'out-json') || DEFAULT_OUT_JSON,
    outMarkdown: readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD,
  };
}

function read(filePath: string): string {
  return fs.readFileSync(path.resolve(ROOT, filePath), 'utf8');
}

function parseWorkflowName(source: string): string {
  const match = source.match(/^\s*name:\s*(.+)\s*$/m);
  return match ? match[1].trim() : '(unnamed)';
}

function hasPushTrigger(source: string): boolean {
  return /^\s*push:\s*$/m.test(source) || /^\s*push:\s*\{/m.test(source);
}

function hasPullRequestTrigger(source: string): boolean {
  return /^\s*pull_request:\s*$/m.test(source) || /^\s*pull_request:\s*\{/m.test(source);
}

function hasJobId(source: string, jobId: string): boolean {
  const pattern = new RegExp(`^\\s{2}${jobId}:\\s*$`, 'm');
  return pattern.test(source);
}

function includesCommand(source: string, snippet: string): boolean {
  return source.includes(snippet);
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# CI Workflow Rationalization Contract');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push('');
  lines.push('## Checks');
  for (const row of payload.checks) {
    lines.push(`- ${row.id}: ${row.ok ? 'pass' : 'fail'} (${row.detail})`);
  }
  lines.push('');
  lines.push('## Workflow Names');
  for (const row of payload.workflow_names) {
    lines.push(`- ${row.file}: ${row.name}`);
  }
  return `${lines.join('\n')}\n`;
}

function run(argv: string[]): number {
  const args = resolveArgs(argv);
  const requiredChecksPath = '.github/workflows/required-checks.yml';
  const ciPath = '.github/workflows/ci.yml';
  const releasePath = '.github/workflows/release.yml';
  const workflowFiles = [requiredChecksPath, ciPath, releasePath];

  const sources = Object.fromEntries(
    workflowFiles.map((filePath) => [filePath, fs.existsSync(path.resolve(ROOT, filePath)) ? read(filePath) : ''])
  );
  const workflowNames = workflowFiles.map((file) => ({
    file,
    name: parseWorkflowName(sources[file]),
  }));

  const uniqueNames = new Set(workflowNames.map((row) => row.name));
  const checks: CheckRow[] = [
    ...workflowFiles.map((filePath) => ({
      id: `workflow_present:${filePath}`,
      ok: Boolean(sources[filePath]),
      detail: sources[filePath] ? 'present' : 'missing',
    })),
    {
      id: 'workflow_names_unique',
      ok: uniqueNames.size === workflowNames.length,
      detail: uniqueNames.size === workflowNames.length ? 'unique' : 'duplicate_workflow_names_detected',
    },
    {
      id: 'required_checks_dispatch_only',
      ok: !hasPushTrigger(sources[requiredChecksPath]) && !hasPullRequestTrigger(sources[requiredChecksPath]),
      detail: 'required-checks should stay manual-only to avoid duplicate push/pr signals',
    },
    {
      id: 'ci_has_push_and_pr',
      ok: hasPushTrigger(sources[ciPath]) && hasPullRequestTrigger(sources[ciPath]),
      detail: 'ci.yml must remain the canonical push/pull_request gate runner',
    },
    ...['typecheck_systems', 'contract_check', 'schema_contract_check', 'js_holdout_audit'].map((jobId) => ({
      id: `required_checks_job:${jobId}`,
      ok: hasJobId(sources[requiredChecksPath], jobId),
      detail: hasJobId(sources[requiredChecksPath], jobId) ? 'present' : 'missing',
    })),
    ...[
      'ops:file-size:gate',
      'ops:srs:full:gate',
      'ops:tooling-registry:contract:guard',
      'ops:policy-debt:summary',
    ].map((snippet) => ({
      id: `ci_invocation:${snippet}`,
      ok: includesCommand(sources[ciPath], snippet),
      detail: includesCommand(sources[ciPath], snippet) ? 'present' : 'missing',
    })),
  ];

  const payload = {
    ok: checks.every((row) => row.ok),
    type: 'ci_workflow_rationalization_contract',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
    },
    workflow_names: workflowNames,
    checks,
    summary: {
      workflow_count: workflowFiles.length,
      failure_count: checks.filter((row) => !row.ok).length,
      pass: checks.every((row) => row.ok),
    },
  };

  writeTextArtifact(args.outMarkdown, toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok: payload.ok,
  });
}

process.exit(run(process.argv.slice(2)));
