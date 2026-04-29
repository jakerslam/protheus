#!/usr/bin/env node
/* eslint-disable no-console */
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_APP = 'client/runtime/systems/ui/infring_static/js/app.ts';
const DEFAULT_PARTS_README = 'client/runtime/systems/ui/infring_static/js/app.ts.parts/README.md';
const DEFAULT_POLICY = 'docs/workspace/shell_source_of_truth_policy.md';
const DEFAULT_INVENTORY = 'core/local/artifacts/shell_duplicate_ts_inventory_current.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_app_surface_ownership_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_APP_SURFACE_OWNERSHIP_GUARD_CURRENT.md';

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  appPath: string;
  partsReadmePath: string;
  policyPath: string;
  inventoryPath: string;
};

type Violation = {
  kind: string;
  path?: string;
  token?: string;
  detail: string;
};

function readArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
    appPath: cleanText(readFlag(argv, 'app') || DEFAULT_APP, 400),
    partsReadmePath: cleanText(readFlag(argv, 'parts-readme') || DEFAULT_PARTS_README, 400),
    policyPath: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY, 400),
    inventoryPath: cleanText(readFlag(argv, 'inventory') || DEFAULT_INVENTORY, 400),
  };
}

function readText(relPath: string): string {
  return readFileSync(resolve(ROOT, relPath), 'utf8');
}

function requireExists(relPath: string, violations: Violation[], kind: string, detail: string): boolean {
  if (existsSync(resolve(ROOT, relPath))) return true;
  violations.push({ kind, path: relPath, detail });
  return false;
}

function requireTokens(relPath: string, source: string, tokens: string[], kind: string, detail: string): Violation[] {
  return tokens
    .filter((token) => !source.includes(token))
    .map((token) => ({ kind, path: relPath, token, detail }));
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell App Surface Ownership Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push(`- inventory_counterparts: ${payload.summary.inventory_counterparts}`);
  lines.push(`- inventory_duplicate_loc_estimate: ${payload.summary.inventory_duplicate_loc_estimate}`);
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) lines.push('- none');
  for (const violation of payload.violations) {
    lines.push(`- ${violation.kind}: ${violation.path || 'unknown'} ${violation.token || ''}`);
  }
  return `${lines.join('\n')}\n`;
}

function run(argv = process.argv.slice(2)): number {
  const args = readArgs(argv);
  const violations: Violation[] = [];

  const pathsReady = [
    requireExists(args.appPath, violations, 'missing_app_surface', 'The canonical assembled app runtime surface must exist.'),
    requireExists(args.partsReadmePath, violations, 'missing_app_parts_readme', 'The app parts decomposition README must exist.'),
    requireExists(args.policyPath, violations, 'missing_shell_source_policy', 'The shell source-of-truth policy must exist.'),
    requireExists(args.inventoryPath, violations, 'missing_duplicate_inventory', 'The duplicate-surface inventory artifact must exist before ownership can be validated.'),
  ].every(Boolean);

  let inventoryCounterparts = 0;
  let inventoryDuplicateLocEstimate = 0;

  if (pathsReady) {
    const app = readText(args.appPath);
    violations.push(
      ...requireTokens(
        args.appPath,
        app,
        [
          'Canonical Shell source-of-truth: assembled runtime app surface.',
          'Decomposition debt lives under ./app.ts.parts/**',
        ],
        'app_surface_missing_canonical_marker',
        'The assembled app runtime file must declare itself as the canonical Shell source-of-truth.',
      ),
    );

    const readme = readText(args.partsReadmePath);
    violations.push(
      ...requireTokens(
        args.partsReadmePath,
        readme,
        [
          '# `app.ts.parts`',
          'Canonical runtime surface: `../app.ts`',
          'Status: decomposition debt only',
          'runtime ownership stays with `../app.ts`',
        ],
        'app_parts_readme_missing_marker',
        'The app parts directory must explicitly declare that it is non-canonical decomposition debt.',
      ),
    );

    const policy = readText(args.policyPath);
    violations.push(
      ...requireTokens(
        args.policyPath,
        policy,
        [
          'canonical assembled files that are still the runtime entry surface during migration, such as `app.ts` and `pages/chat.ts`',
        ],
        'shell_policy_missing_app_ownership_rule',
        'The shell source-of-truth policy must explicitly classify the app assembled surface.',
      ),
    );

    const inventory = JSON.parse(readText(args.inventoryPath));
    const groups = Array.isArray(inventory && inventory.duplicate_groups) ? inventory.duplicate_groups : [];
    const appGroup = groups.find(
      (row: any) =>
        row &&
        row.kind === 'assembled_vs_parts' &&
        row.canonical_path === args.appPath,
    );
    if (!appGroup) {
      violations.push({
        kind: 'duplicate_inventory_missing_app_group',
        path: args.inventoryPath,
        detail: 'The duplicate-surface inventory must classify app.ts against app.ts.parts/** as one logical surface.',
      });
    } else {
      inventoryCounterparts = Array.isArray(appGroup.counterpart_paths) ? appGroup.counterpart_paths.length : 0;
      inventoryDuplicateLocEstimate = Number(appGroup.duplicate_loc_estimate || 0);
      if (inventoryCounterparts <= 0) {
        violations.push({
          kind: 'duplicate_inventory_app_group_empty',
          path: args.inventoryPath,
          detail: 'The duplicate-surface inventory found app.ts but no app.ts.parts/** counterparts.',
        });
      }
    }
  }

  const payload = {
    ok: violations.length === 0,
    type: 'shell_app_surface_ownership_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      app_path: args.appPath,
      parts_readme_path: args.partsReadmePath,
      policy_path: args.policyPath,
      inventory_path: args.inventoryPath,
    },
    summary: {
      violations: violations.length,
      inventory_counterparts: inventoryCounterparts,
      inventory_duplicate_loc_estimate: inventoryDuplicateLocEstimate,
    },
    violations,
  };

  writeTextArtifact(args.outMarkdown, markdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok: payload.ok,
  });
}

process.exit(run(process.argv.slice(2)));
