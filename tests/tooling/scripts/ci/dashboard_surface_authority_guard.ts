#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const UI_ROOT = path.resolve(ROOT, 'client/runtime/systems/ui');
const PRIMARY_ROOT_NAME = 'infring_static';
const PRIMARY_ROOT = path.resolve(UI_ROOT, PRIMARY_ROOT_NAME);
const HOST_PATH = path.resolve(ROOT, 'adapters/runtime/infring_dashboard.ts');
const DIST_BUILD_PATH = path.resolve(ROOT, 'tests/tooling/scripts/ci/build_dashboard_dist.ts');
const DEFAULT_OUT_JSON = 'core/local/artifacts/dashboard_surface_authority_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/DASHBOARD_SURFACE_AUTHORITY_GUARD_CURRENT.md';
const FORBIDDEN_UI_ROOTS = [
  'dashboard_sveltekit',
  'legacy_dashboard',
  'reference_runtime_dashboard',
  'control_runtime_dashboard',
  'dashboard_legacy',
  'deprecated_dashboard',
];

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

function readText(filePath: string): string {
  return fs.readFileSync(filePath, 'utf8');
}

function uiRootDirectories(): string[] {
  if (!fs.existsSync(UI_ROOT)) return [];
  return fs
    .readdirSync(UI_ROOT, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort((a, b) => a.localeCompare(b, 'en'));
}

function staticFiles(root: string): string[] {
  const out: string[] = [];
  if (!fs.existsSync(root)) return out;
  const walk = (dir: string) => {
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        walk(fullPath);
        continue;
      }
      if (!/\.(ts|css|html)$/.test(entry.name)) continue;
      out.push(path.relative(ROOT, fullPath));
    }
  };
  walk(root);
  return out.sort((a, b) => a.localeCompare(b, 'en'));
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Dashboard Surface Authority Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- ui_roots_detected: ${payload.summary.ui_roots_detected}`);
  lines.push(`- dashboard_asset_files: ${payload.summary.dashboard_asset_files}`);
  lines.push(`- forbidden_surface_directories: ${payload.summary.forbidden_surface_directories}`);
  lines.push(`- redirect_alias_handlers: ${payload.summary.redirect_alias_handlers}`);
  lines.push(`- retired_alias_guard_present: ${payload.summary.retired_alias_guard_present}`);
  lines.push(`- svelte_dashboard_packaged: ${payload.summary.svelte_dashboard_packaged}`);
  lines.push('');
  lines.push('## UI Roots');
  if (!payload.ui_roots.length) lines.push('- none');
  else payload.ui_roots.forEach((row: string) => lines.push(`- ${row}`));
  lines.push('');
  lines.push('## Forbidden Directories');
  if (!payload.forbidden_directories.length) lines.push('- none');
  else payload.forbidden_directories.forEach((row: string) => lines.push(`- ${row}`));
  return `${lines.join('\n')}\n`;
}

function run(argv: string[]): number {
  const args = resolveArgs(argv);
  const uiRoots = uiRootDirectories();
  const forbiddenDirectories = FORBIDDEN_UI_ROOTS
    .filter((name) => fs.existsSync(path.resolve(UI_ROOT, name)))
    .map((name) => path.posix.join('client/runtime/systems/ui', name));
  const extraRoots = uiRoots.filter((name) => name !== PRIMARY_ROOT_NAME);
  const hostSource = readText(HOST_PATH);
  const distBuildSource = readText(DIST_BUILD_PATH);
  const redirectAliasHandlers = hostSource.includes("location: `/dashboard${search}`") ? 1 : 0;
  const retiredAliasGuardPresent = hostSource.includes('dashboard_surface_retired');
  const svelteDashboardPackaged = distBuildSource.includes('dashboard_sveltekit');
  const dashboardAssetFiles = staticFiles(PRIMARY_ROOT);
  const ok =
    fs.existsSync(PRIMARY_ROOT) &&
    uiRoots.length === 1 &&
    uiRoots[0] === PRIMARY_ROOT_NAME &&
    forbiddenDirectories.length === 0 &&
    extraRoots.length === 0 &&
    retiredAliasGuardPresent &&
    redirectAliasHandlers === 0 &&
    !svelteDashboardPackaged;

  const payload = {
    ok,
    type: 'dashboard_surface_authority_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
    },
    ui_roots: uiRoots,
    forbidden_directories: forbiddenDirectories,
    summary: {
      pass: ok,
      ui_roots_detected: uiRoots.length,
      dashboard_asset_files: dashboardAssetFiles.length,
      forbidden_surface_directories: forbiddenDirectories.length,
      redirect_alias_handlers: redirectAliasHandlers,
      retired_alias_guard_present: retiredAliasGuardPresent,
      svelte_dashboard_packaged: svelteDashboardPackaged,
    },
  };

  writeTextArtifact(args.outMarkdown, toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok,
  });
}

process.exit(run(process.argv.slice(2)));
