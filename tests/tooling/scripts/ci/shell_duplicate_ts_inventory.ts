#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision, trackedFiles } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_duplicate_ts_inventory_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_DUPLICATE_TS_INVENTORY_CURRENT.md';
const SHELL_PREFIX = 'client/runtime/systems/ui/infring_static/';

type ScriptArgs = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
};

type DuplicateGroup = {
  kind: 'assembled_vs_parts' | 'svelte_source_vs_bundle';
  canonical_path: string;
  canonical_nonblank_loc: number;
  counterpart_paths: string[];
  counterpart_nonblank_loc: number;
  duplicate_loc_estimate: number;
};

function resolveArgs(argv: string[]): ScriptArgs {
  return {
    strict: argv.includes('--strict') || parseBool(readFlag(argv, 'strict'), false),
    outJson: readFlag(argv, 'out-json') || DEFAULT_OUT_JSON,
    outMarkdown: readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN,
  };
}

function isProductionTs(file: string): boolean {
  if (!/\.(ts|tsx)$/.test(file)) return false;
  if (!file.startsWith(SHELL_PREFIX)) return false;
  if (file.includes('/vendor/')) return false;
  if (file.endsWith('.min.ts') || file.endsWith('.min.tsx')) return false;
  if (file.endsWith('.d.ts')) return false;
  if (/[-_]test\./.test(file) || /\.test\./.test(file) || /\.spec\./.test(file)) return false;
  if (file.includes('/tests/') || file.includes('/test/') || file.includes('/__tests__/')) return false;
  return true;
}

function nonblankLoc(filePath: string): number {
  const abs = path.resolve(ROOT, filePath);
  const raw = fs.readFileSync(abs, 'utf8');
  return raw.split(/\r?\n/).filter((line) => line.trim() !== '').length;
}

function sumNonblankLoc(paths: string[], counts: Map<string, number>): number {
  return paths.reduce((sum, filePath) => sum + (counts.get(filePath) || 0), 0);
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell Duplicate TS Inventory');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- tracked_shell_ts_files: ${payload.summary.tracked_shell_ts_files}`);
  lines.push(`- tracked_shell_ts_nonblank_loc: ${payload.summary.tracked_shell_ts_nonblank_loc}`);
  lines.push(`- assembled_vs_parts_groups: ${payload.summary.assembled_vs_parts_groups}`);
  lines.push(`- assembled_vs_parts_duplicate_loc_estimate: ${payload.summary.assembled_vs_parts_duplicate_loc_estimate}`);
  lines.push(`- svelte_source_vs_bundle_groups: ${payload.summary.svelte_source_vs_bundle_groups}`);
  lines.push(`- svelte_source_vs_bundle_duplicate_loc_estimate: ${payload.summary.svelte_source_vs_bundle_duplicate_loc_estimate}`);
  lines.push(`- total_duplicate_loc_estimate: ${payload.summary.total_duplicate_loc_estimate}`);
  lines.push('');
  lines.push('## Top Duplicate Groups');
  for (const row of payload.top_duplicate_groups) {
    lines.push(
      `- [${row.kind}] ${row.canonical_path} :: duplicate_loc_estimate=${row.duplicate_loc_estimate}, canonical_loc=${row.canonical_nonblank_loc}, counterpart_loc=${row.counterpart_nonblank_loc}, counterparts=${row.counterpart_paths.length}`,
    );
  }
  lines.push('');
  lines.push('## Orphans');
  lines.push(`- parts_without_assembled_count: ${payload.summary.parts_without_assembled_count}`);
  lines.push(`- svelte_sources_without_bundle_count: ${payload.summary.svelte_sources_without_bundle_count}`);
  lines.push(`- bundles_without_svelte_source_count: ${payload.summary.bundles_without_svelte_source_count}`);
  return `${lines.join('\n')}\n`;
}

function run(argv: string[]): number {
  const args = resolveArgs(argv);
  const files = trackedFiles(ROOT)
    .map((file) => file.replace(/\\/g, '/'))
    .filter(isProductionTs);
  const counts = new Map<string, number>();
  for (const file of files) counts.set(file, nonblankLoc(file));

  const assembledGroups: DuplicateGroup[] = [];
  const partsWithoutAssembled: string[] = [];
  const partFiles = files.filter((file) => file.includes('.parts/'));
  const groupedParts = new Map<string, string[]>();
  for (const file of partFiles) {
    const [base] = file.split('.parts/');
    const key = base;
    const rows = groupedParts.get(key) || [];
    rows.push(file);
    groupedParts.set(key, rows);
  }
  for (const [assembledPath, counterpartPaths] of groupedParts.entries()) {
    if (!counts.has(assembledPath)) {
      partsWithoutAssembled.push(...counterpartPaths);
      continue;
    }
    const canonicalLoc = counts.get(assembledPath) || 0;
    const counterpartLoc = sumNonblankLoc(counterpartPaths, counts);
    assembledGroups.push({
      kind: 'assembled_vs_parts',
      canonical_path: assembledPath,
      canonical_nonblank_loc: canonicalLoc,
      counterpart_paths: counterpartPaths.sort(),
      counterpart_nonblank_loc: counterpartLoc,
      duplicate_loc_estimate: Math.min(canonicalLoc, counterpartLoc),
    });
  }

  const svelteGroups: DuplicateGroup[] = [];
  const svelteSourcesWithoutBundle: string[] = [];
  const bundlesWithoutSvelteSource: string[] = [];
  const svelteSources = files.filter((file) => file.endsWith('_svelte_source.ts'));
  const bundles = new Set(files.filter((file) => file.endsWith('.bundle.ts')));
  const matchedBundles = new Set<string>();
  for (const sourcePath of svelteSources) {
    const bundlePath = sourcePath.replace(/_svelte_source\.ts$/, '.bundle.ts');
    if (!bundles.has(bundlePath)) {
      svelteSourcesWithoutBundle.push(sourcePath);
      continue;
    }
    matchedBundles.add(bundlePath);
    const canonicalLoc = counts.get(sourcePath) || 0;
    const counterpartLoc = counts.get(bundlePath) || 0;
    svelteGroups.push({
      kind: 'svelte_source_vs_bundle',
      canonical_path: sourcePath,
      canonical_nonblank_loc: canonicalLoc,
      counterpart_paths: [bundlePath],
      counterpart_nonblank_loc: counterpartLoc,
      duplicate_loc_estimate: Math.min(canonicalLoc, counterpartLoc),
    });
  }
  for (const bundlePath of bundles) {
    if (!matchedBundles.has(bundlePath)) bundlesWithoutSvelteSource.push(bundlePath);
  }

  const allGroups = [...assembledGroups, ...svelteGroups].sort(
    (a, b) => b.duplicate_loc_estimate - a.duplicate_loc_estimate,
  );
  const trackedShellTsNonblankLoc = files.reduce((sum, file) => sum + (counts.get(file) || 0), 0);
  const assembledDuplicateLoc = assembledGroups.reduce((sum, row) => sum + row.duplicate_loc_estimate, 0);
  const svelteDuplicateLoc = svelteGroups.reduce((sum, row) => sum + row.duplicate_loc_estimate, 0);

  const payload = {
    ok: true,
    type: 'shell_duplicate_ts_inventory',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
      shell_prefix: SHELL_PREFIX,
    },
    summary: {
      tracked_shell_ts_files: files.length,
      tracked_shell_ts_nonblank_loc: trackedShellTsNonblankLoc,
      assembled_vs_parts_groups: assembledGroups.length,
      assembled_vs_parts_duplicate_loc_estimate: assembledDuplicateLoc,
      svelte_source_vs_bundle_groups: svelteGroups.length,
      svelte_source_vs_bundle_duplicate_loc_estimate: svelteDuplicateLoc,
      total_duplicate_loc_estimate: assembledDuplicateLoc + svelteDuplicateLoc,
      parts_without_assembled_count: partsWithoutAssembled.length,
      svelte_sources_without_bundle_count: svelteSourcesWithoutBundle.length,
      bundles_without_svelte_source_count: bundlesWithoutSvelteSource.length,
    },
    top_duplicate_groups: allGroups.slice(0, 25),
    duplicate_groups: allGroups,
    orphans: {
      parts_without_assembled: partsWithoutAssembled.sort(),
      svelte_sources_without_bundle: svelteSourcesWithoutBundle.sort(),
      bundles_without_svelte_source: bundlesWithoutSvelteSource.sort(),
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
