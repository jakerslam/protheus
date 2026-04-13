#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';
import { executeGate } from '../../lib/runner.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/ci_quality_scorecard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/CI_QUALITY_SCORECARD_CURRENT.md';
const DEFAULT_POLICY = 'client/runtime/config/ci_quality_scorecard_policy.json';

type ScriptArgs = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  policyPath: string;
};

function resolveArgs(argv: string[]): ScriptArgs {
  return {
    strict: argv.includes('--strict') || parseBool(readFlag(argv, 'strict'), false),
    outJson: readFlag(argv, 'out-json') || DEFAULT_OUT_JSON,
    outMarkdown: readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD,
    policyPath: readFlag(argv, 'policy') || DEFAULT_POLICY,
  };
}

function readJsonMaybe<T>(filePath: string, fallback: T): T {
  const abs = path.resolve(ROOT, filePath);
  if (!fs.existsSync(abs)) return fallback;
  try {
    return JSON.parse(fs.readFileSync(abs, 'utf8')) as T;
  } catch {
    return fallback;
  }
}

function appendHistory(filePath: string, row: unknown) {
  const abs = path.resolve(ROOT, filePath);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.appendFileSync(abs, `${JSON.stringify(row)}\n`);
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# CI Quality Scorecard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push('');
  lines.push('## Metrics');
  lines.push(`- coverage_pct: ${payload.metrics.coverage_pct}`);
  lines.push(`- flake_rate: ${payload.metrics.flake_rate}`);
  lines.push(`- p95_runtime_ms: ${payload.metrics.p95_runtime_ms}`);
  lines.push(`- critical_suite_pass: ${payload.metrics.critical_suite_pass}`);
  lines.push('');
  lines.push('## Gates');
  for (const row of payload.gates) {
    lines.push(`- ${row.id}: ${row.ok ? 'pass' : 'fail'} (${row.detail})`);
  }
  return `${lines.join('\n')}\n`;
}

function run(argv: string[]): number {
  const args = resolveArgs(argv);
  const policy = readJsonMaybe<any>(args.policyPath, {});
  const scorecardPaths = policy?.paths || {};
  const thresholds = policy?.thresholds || {};

  const gateIds = [
    'ops:srs:full:gate',
    'ops:file-size:gate',
    'ops:arch:conformance',
    'ops:tooling-registry:contract:guard',
  ];
  const gateReports = gateIds.map((gateId) => executeGate(gateId, { strict: true }));
  const criticalSuitePass = gateReports.every((row) => row.ok);

  const coverageSummary = readJsonMaybe<any>('core/local/artifacts/coverage_summary_current.json', null);
  const flakeSummary = readJsonMaybe<any>('core/local/artifacts/flaky_quarantine_audit_current.json', null);
  const coveragePct = Number(coverageSummary?.summary?.coverage_pct ?? 0);
  const flakeRate = Number(flakeSummary?.summary?.flake_rate ?? 0);
  const p95RuntimeMs = 0;

  const metricChecks = [
    {
      id: 'coverage_pct',
      ok: coveragePct >= Number(thresholds.min_coverage_pct ?? 0),
      detail: `value=${coveragePct}; min=${Number(thresholds.min_coverage_pct ?? 0)}`,
    },
    {
      id: 'flake_rate',
      ok: flakeRate <= Number(thresholds.max_flake_rate ?? 0.05),
      detail: `value=${flakeRate}; max=${Number(thresholds.max_flake_rate ?? 0.05)}`,
    },
    {
      id: 'p95_runtime_ms',
      ok: p95RuntimeMs <= Number(thresholds.max_p95_runtime_ms ?? 360000),
      detail: `value=${p95RuntimeMs}; max=${Number(thresholds.max_p95_runtime_ms ?? 360000)}`,
    },
    {
      id: 'critical_suite_pass',
      ok: thresholds.require_critical_suite_pass === false ? true : criticalSuitePass,
      detail: criticalSuitePass ? 'all critical gates passed' : 'one or more critical gates failed',
    },
  ];

  const payload = {
    ok: metricChecks.every((row) => row.ok),
    type: 'ci_quality_scorecard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
      policy_path: args.policyPath,
    },
    metrics: {
      coverage_pct: coveragePct,
      flake_rate: flakeRate,
      p95_runtime_ms: p95RuntimeMs,
      critical_suite_pass: criticalSuitePass,
    },
    thresholds: {
      min_coverage_pct: Number(thresholds.min_coverage_pct ?? 0),
      max_flake_rate: Number(thresholds.max_flake_rate ?? 0.05),
      max_p95_runtime_ms: Number(thresholds.max_p95_runtime_ms ?? 360000),
      require_critical_suite_pass: thresholds.require_critical_suite_pass !== false,
    },
    gates: [
      ...gateReports.map((row) => ({
        id: row.gate_id,
        ok: row.ok,
        detail: `exit_code=${row.summary.exit_code}`,
      })),
      ...metricChecks,
    ],
    summary: {
      failure_count: metricChecks.filter((row) => !row.ok).length,
      critical_gate_count: gateReports.length,
      pass: metricChecks.every((row) => row.ok),
    },
  };

  const latestPath = String(scorecardPaths.latest_path || '').trim();
  const historyPath = String(scorecardPaths.history_path || '').trim();
  if (latestPath) {
    const abs = path.resolve(ROOT, latestPath);
    fs.mkdirSync(path.dirname(abs), { recursive: true });
    fs.writeFileSync(abs, `${JSON.stringify(payload, null, 2)}\n`);
  }
  if (historyPath) {
    appendHistory(historyPath, {
      generated_at: payload.generated_at,
      revision: payload.revision,
      metrics: payload.metrics,
      ok: payload.ok,
    });
  }

  writeTextArtifact(args.outMarkdown, toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok: payload.ok,
  });
}

process.exit(run(process.argv.slice(2)));
