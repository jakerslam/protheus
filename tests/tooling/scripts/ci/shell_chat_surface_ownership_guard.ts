#!/usr/bin/env node
/* eslint-disable no-console */
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CHAT = 'client/runtime/systems/ui/infring_static/js/pages/chat.ts';
const DEFAULT_PARTS_README = 'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/README.md';
const DEFAULT_POLICY = 'docs/workspace/shell_source_of_truth_policy.md';
const DEFAULT_INVENTORY = 'core/local/artifacts/shell_duplicate_ts_inventory_current.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_chat_surface_ownership_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_CHAT_SURFACE_OWNERSHIP_GUARD_CURRENT.md';

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  chatPath: string;
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
    chatPath: cleanText(readFlag(argv, 'chat') || DEFAULT_CHAT, 400),
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
  lines.push('# Shell Chat Surface Ownership Guard');
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
    requireExists(args.chatPath, violations, 'missing_chat_surface', 'The canonical assembled chat runtime surface must exist.'),
    requireExists(args.partsReadmePath, violations, 'missing_chat_parts_readme', 'The chat parts decomposition README must exist.'),
    requireExists(args.policyPath, violations, 'missing_shell_source_policy', 'The shell source-of-truth policy must exist.'),
    requireExists(args.inventoryPath, violations, 'missing_duplicate_inventory', 'The duplicate-surface inventory artifact must exist before ownership can be validated.'),
  ].every(Boolean);

  let inventoryCounterparts = 0;
  let inventoryDuplicateLocEstimate = 0;

  if (pathsReady) {
    const chat = readText(args.chatPath);
    violations.push(
      ...requireTokens(
        args.chatPath,
        chat,
        [
          'Canonical Shell source-of-truth: assembled runtime chat surface.',
          'Decomposition debt lives under ./chat.ts.parts/**',
        ],
        'chat_surface_missing_canonical_marker',
        'The assembled chat runtime file must declare itself as the canonical Shell source-of-truth.',
      ),
    );

    const readme = readText(args.partsReadmePath);
    violations.push(
      ...requireTokens(
        args.partsReadmePath,
        readme,
        [
          '# `chat.ts.parts`',
          'Canonical runtime surface: `../chat.ts`',
          'Status: decomposition debt only',
          'runtime ownership stays with `../chat.ts`',
        ],
        'chat_parts_readme_missing_marker',
        'The chat parts directory must explicitly declare that it is non-canonical decomposition debt.',
      ),
    );

    const policy = readText(args.policyPath);
    violations.push(
      ...requireTokens(
        args.policyPath,
        policy,
        [
          'canonical assembled files that are still the runtime entry surface during migration, such as `app.ts` and `pages/chat.ts`',
          '- `pages/chat.ts` and `pages/chat.ts.parts/**` are one logical surface, not two',
        ],
        'shell_policy_missing_chat_ownership_rule',
        'The shell source-of-truth policy must explicitly classify the chat assembled surface and parts mirror.',
      ),
    );

    const inventory = JSON.parse(readText(args.inventoryPath));
    const groups = Array.isArray(inventory && inventory.duplicate_groups) ? inventory.duplicate_groups : [];
    const chatGroup = groups.find(
      (row: any) =>
        row &&
        row.kind === 'assembled_vs_parts' &&
        row.canonical_path === args.chatPath,
    );
    if (!chatGroup) {
      violations.push({
        kind: 'duplicate_inventory_missing_chat_group',
        path: args.inventoryPath,
        detail: 'The duplicate-surface inventory must classify chat.ts against chat.ts.parts/** as one logical surface.',
      });
    } else {
      inventoryCounterparts = Array.isArray(chatGroup.counterpart_paths) ? chatGroup.counterpart_paths.length : 0;
      inventoryDuplicateLocEstimate = Number(chatGroup.duplicate_loc_estimate || 0);
      if (inventoryCounterparts <= 0) {
        violations.push({
          kind: 'duplicate_inventory_chat_group_empty',
          path: args.inventoryPath,
          detail: 'The duplicate-surface inventory found chat.ts but no chat.ts.parts/** counterparts.',
        });
      }
    }
  }

  const payload = {
    ok: violations.length === 0,
    type: 'shell_chat_surface_ownership_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      chat_path: args.chatPath,
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
