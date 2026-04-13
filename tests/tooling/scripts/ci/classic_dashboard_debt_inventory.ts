#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { execSync } from 'node:child_process';
import { parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/classic_dashboard_debt_inventory_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/CLASSIC_DASHBOARD_DEBT_INVENTORY_CURRENT.md';

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

function read(filePath: string): string {
  return fs.readFileSync(path.resolve(ROOT, filePath), 'utf8');
}

function lineCount(filePath: string): number {
  return read(filePath).split(/\r?\n/).length;
}

function rgCount(pattern: string, roots: string[]): number {
  try {
    const output = execSync(
      `rg -n ${JSON.stringify(pattern)} ${roots.map((row) => JSON.stringify(row)).join(' ')}`,
      { cwd: ROOT, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }
    );
    return output.split('\n').filter(Boolean).length;
  } catch {
    return 0;
  }
}

function staticFiles(root: string): string[] {
  try {
    const output = execSync(
      `rg --files ${JSON.stringify(root)} | rg '\\.(ts|css|html)$'`,
      { cwd: ROOT, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }
    );
    return output.split('\n').map((row) => row.trim()).filter(Boolean).sort();
  } catch {
    return [];
  }
}

function parseDashboardModes(source: string) {
  const rows = [...source.matchAll(/\{\s*key:\s*'([^']+)'[\s\S]*?mode:\s*'([^']+)'/g)];
  return rows.map((row) => ({ key: row[1], mode: row[2] }));
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Classic Dashboard Debt Inventory');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- native_pages: ${payload.summary.native_pages}`);
  lines.push(`- legacy_pages: ${payload.summary.legacy_pages}`);
  lines.push(`- classic_asset_files: ${payload.summary.classic_asset_files}`);
  lines.push(`- classic_href_references: ${payload.summary.classic_href_references}`);
  lines.push(`- embedded_fallback_references: ${payload.summary.embedded_fallback_references}`);
  lines.push('');
  lines.push('## Top Classic Files');
  for (const row of payload.top_classic_files) {
    lines.push(`- ${row.path}: ${row.lines} lines`);
  }
  return `${lines.join('\n')}\n`;
}

function run(argv: string[]): number {
  const args = resolveArgs(argv);
  const dashboardPath = 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/dashboard.ts';
  const dashboardSource = read(dashboardPath);
  const pageModes = parseDashboardModes(dashboardSource);
  const classicRoot = 'client/runtime/systems/ui/infring_static';
  const classicFiles = staticFiles(classicRoot);
  const topClassicFiles = classicFiles
    .map((file) => ({ path: file, lines: lineCount(file) }))
    .sort((a, b) => b.lines - a.lines || a.path.localeCompare(b.path))
    .slice(0, 15);

  const scanRoots = [
    'client/runtime/systems/ui/dashboard_sveltekit/src',
    'client/runtime/systems/ui/infring_dashboard.ts',
  ];
  const payload = {
    ok: true,
    type: 'classic_dashboard_debt_inventory',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
    },
    summary: {
      native_pages: pageModes.filter((row) => row.mode === 'native').length,
      legacy_pages: pageModes.filter((row) => row.mode === 'legacy').length,
      classic_asset_files: classicFiles.length,
      classic_href_references: rgCount('dashboardClassicHref', scanRoots),
      embedded_fallback_references: rgCount('dashboardEmbeddedFallbackHref|dashboard-classic\\?embed=1|classic fallback', scanRoots),
    },
    page_modes: pageModes,
    top_classic_files: topClassicFiles,
  };

  writeTextArtifact(args.outMarkdown, toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok: payload.ok,
  });
}

process.exit(run(process.argv.slice(2)));
