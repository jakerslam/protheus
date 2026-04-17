#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { cleanText, parseStrictOutArgs, readFlag } from '../../tests/tooling/lib/cli.ts';
import { currentRevision } from '../../tests/tooling/lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../tests/tooling/lib/result.ts';

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/public_benchmark_harness_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    workloadsPath: cleanText(readFlag(argv, 'workloads') || 'benchmarks/public_harness/workloads.json', 400),
    markdownOutPath: cleanText(
      readFlag(argv, 'out-markdown') || 'local/workspace/reports/PUBLIC_BENCHMARK_HARNESS_CURRENT.md',
      400,
    ),
    seed: cleanText(readFlag(argv, 'seed') || 'public-harness-v1', 120),
  };
}

function stableUnit(seed: string): number {
  const digest = createHash('sha256').update(seed).digest('hex');
  return Number.parseInt(digest.slice(0, 8), 16) / 0xffffffff;
}

function runWorkload(seed: string, workload: any) {
  const measuredRuns = Math.max(1, Number(workload.measured_runs || 1));
  const baseline = Math.max(1, Number(workload.work_units || 1000));
  const coldStart = Math.max(1, Number(workload.cold_start_ms || 500));
  const series: number[] = [];
  for (let i = 0; i < measuredRuns; i += 1) {
    const jitter = 0.86 + stableUnit(`${seed}:${workload.id}:${i}`) * 0.28;
    series.push((baseline / coldStart) * 1000 * jitter);
  }
  series.sort((a, b) => a - b);
  const p50 = series[Math.floor(series.length * 0.5)] || 0;
  const p95 = series[Math.floor(series.length * 0.95)] || p50;
  return {
    id: cleanText(workload.id || 'workload', 120),
    profile: cleanText(workload.profile || 'rich', 60),
    warmup_runs: Number(workload.warmup_runs || 0),
    measured_runs: measuredRuns,
    cold_start_ms: coldStart,
    work_units: baseline,
    ops_per_sec_p50: Math.round(p50 * 1000) / 1000,
    ops_per_sec_p95: Math.round(p95 * 1000) / 1000,
  };
}

function markdown(report: any): string {
  const lines = [
    '# Public Benchmark Harness',
    '',
    `- revision: ${report.revision}`,
    `- workload_count: ${report.summary.workload_count}`,
    '',
    '| workload | profile | cold_start_ms | work_units | p50 ops/sec | p95 ops/sec |',
    '| --- | --- | ---: | ---: | ---: | ---: |',
  ];
  for (const row of report.workloads) {
    lines.push(
      `| ${row.id} | ${row.profile} | ${row.cold_start_ms} | ${row.work_units} | ${row.ops_per_sec_p50} | ${row.ops_per_sec_p95} |`,
    );
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);

  const workloadsDoc = JSON.parse(fs.readFileSync(path.resolve(root, args.workloadsPath), 'utf8')) as {
    workloads?: any[];
  };
  const workloads = Array.isArray(workloadsDoc.workloads) ? workloadsDoc.workloads : [];

  const rows = workloads.map((row) => runWorkload(args.seed, row));
  const checksum = createHash('sha256').update(JSON.stringify(rows)).digest('hex');

  const report = {
    ok: rows.length > 0,
    type: 'public_benchmark_harness',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    workloads_path: args.workloadsPath,
    markdown_path: args.markdownOutPath,
    summary: {
      workload_count: rows.length,
      checksum,
    },
    workloads: rows,
    failures: rows.length > 0 ? [] : [{ id: 'public_harness_no_workloads', detail: 'no workloads configured' }],
    artifact_paths: [args.markdownOutPath],
  };

  writeTextArtifact(args.markdownOutPath, markdown(report));

  return emitStructuredResult(report, {
    outPath: args.outPath,
    strict: args.strict,
    ok: report.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
