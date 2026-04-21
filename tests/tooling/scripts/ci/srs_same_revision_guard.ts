#!/usr/bin/env node
/* eslint-disable no-console */
import { execSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/srs_same_revision_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/SRS_SAME_REVISION_GUARD_CURRENT.md';
const SRS_PATH = 'docs/workspace/SRS.md';
const PACKAGE_JSON_PATH = 'package.json';
const TOOLING_REGISTRY_PATH = 'tests/tooling/config/tooling_gate_registry.json';

type DiffRow = {
  status: string;
  oldPath: string;
  newPath: string;
};

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  baseRef: string;
};

function parseArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, {
    strict: false,
    out: DEFAULT_OUT_JSON,
  });
  return {
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD, 400),
    baseRef: cleanText(readFlag(argv, 'base') || process.env.INFRING_SRS_GUARD_BASE || '', 200),
  };
}

function runGit(command: string): string {
  return execSync(command, {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    maxBuffer: 32 * 1024 * 1024,
  });
}

function firstNonEmpty(values: Array<string | undefined>): string {
  for (const value of values) {
    const cleaned = cleanText(value || '', 200);
    if (cleaned) return cleaned;
  }
  return '';
}

function resolveBaseRef(explicitBase: string): string {
  const candidate = firstNonEmpty([
    explicitBase,
    process.env.GITHUB_BASE_REF ? `origin/${process.env.GITHUB_BASE_REF}` : '',
  ]);
  if (candidate) {
    try {
      const mergeBase = cleanText(runGit(`git merge-base HEAD ${candidate}`), 120);
      if (mergeBase) return mergeBase;
    } catch {
      // fallback below
    }
  }
  try {
    const fallback = cleanText(runGit('git rev-parse HEAD~1'), 120);
    if (fallback) return fallback;
  } catch {
    // fallback below
  }
  return cleanText(runGit('git rev-parse HEAD'), 120);
}

function parseDiffRows(baseRef: string): DiffRow[] {
  const raw = runGit(`git diff --name-status --find-renames ${baseRef}...HEAD`);
  return String(raw)
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const parts = line.split('\t');
      const status = cleanText(parts[0] || '', 12);
      if (status.startsWith('R') || status.startsWith('C')) {
        return {
          status,
          oldPath: cleanText(parts[1] || '', 300),
          newPath: cleanText(parts[2] || '', 300),
        };
      }
      const pathValue = cleanText(parts[1] || '', 300);
      return {
        status,
        oldPath: pathValue,
        newPath: pathValue,
      };
    });
}

function isNetNewSourcePath(filePath: string): boolean {
  const normalized = cleanText(filePath || '', 400).replace(/^\.?\//, '');
  if (!normalized) return false;
  const ext = path.extname(normalized).toLowerCase();
  if (!(ext === '.rs' || ext === '.ts' || ext === '.tsx')) return false;
  if (normalized.startsWith('core/')) return true;
  if (normalized.startsWith('surface/')) return true;
  if (normalized.startsWith('client/runtime/systems/')) return true;
  if (normalized.startsWith('adapters/')) return true;
  if (normalized.startsWith('tests/tooling/scripts/')) return true;
  if (normalized.startsWith('packages/')) return true;
  return false;
}

function readJsonAtRef<T>(ref: string, filePath: string): T | null {
  try {
    const raw = runGit(`git show ${ref}:${filePath}`);
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

function addedPackageScripts(baseRef: string): string[] {
  const basePkg = readJsonAtRef<{ scripts?: Record<string, string> }>(baseRef, PACKAGE_JSON_PATH);
  const headPkg = readJsonAtRef<{ scripts?: Record<string, string> }>('HEAD', PACKAGE_JSON_PATH);
  const baseScripts = new Set(Object.keys(basePkg?.scripts || {}));
  const headScripts = Object.keys(headPkg?.scripts || {});
  return headScripts.filter((scriptId) => !baseScripts.has(scriptId)).sort();
}

function addedToolingGateIds(baseRef: string): string[] {
  const baseRegistry = readJsonAtRef<{ gates?: Record<string, unknown> }>(baseRef, TOOLING_REGISTRY_PATH);
  const headRegistry = readJsonAtRef<{ gates?: Record<string, unknown> }>('HEAD', TOOLING_REGISTRY_PATH);
  const baseIds = new Set(Object.keys(baseRegistry?.gates || {}));
  const headIds = Object.keys(headRegistry?.gates || {});
  return headIds.filter((gateId) => !baseIds.has(gateId)).sort();
}

function srsHasRowDelta(baseRef: string): boolean {
  try {
    const diff = runGit(`git diff --unified=0 ${baseRef}...HEAD -- ${SRS_PATH}`);
    return diff
      .split(/\r?\n/)
      .some((line) => line.startsWith('+|') && /^\+\|\s*V[0-9A-Z._-]+\s*\|/.test(line));
  } catch {
    return false;
  }
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# SRS Same-Revision Guard');
  lines.push('');
  lines.push(`- generated_at: ${payload.generated_at}`);
  lines.push(`- revision: ${payload.revision}`);
  lines.push(`- base_ref: ${payload.base_ref}`);
  lines.push(`- strict: ${payload.strict}`);
  lines.push(`- pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- changed_files: ${payload.summary.changed_files}`);
  lines.push(`- net_new_source_files: ${payload.summary.net_new_source_files}`);
  lines.push(`- added_package_scripts: ${payload.summary.added_package_scripts}`);
  lines.push(`- added_tooling_gate_ids: ${payload.summary.added_tooling_gate_ids}`);
  lines.push(`- requires_srs_revision_update: ${payload.summary.requires_srs_revision_update}`);
  lines.push(`- srs_changed: ${payload.summary.srs_changed}`);
  lines.push(`- srs_row_delta_present: ${payload.summary.srs_row_delta_present}`);
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push('');
  lines.push('## Violations');
  if (!Array.isArray(payload.violations) || payload.violations.length === 0) {
    lines.push('- none');
  } else {
    payload.violations.forEach((item: any) => {
      lines.push(`- ${cleanText(item?.id || 'violation', 120)}: ${cleanText(item?.detail || '', 240)}`);
    });
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const baseRef = resolveBaseRef(args.baseRef);
  const diffRows = parseDiffRows(baseRef);

  const changedPaths = diffRows
    .map((row) => row.newPath)
    .filter(Boolean)
    .map((value) => value.replace(/^\.?\//, ''));
  const changedSet = new Set(changedPaths);
  const srsChanged = changedSet.has(SRS_PATH);

  const addedRows = diffRows.filter((row) => cleanText(row.status, 8).startsWith('A'));
  const netNewSourceFiles = addedRows
    .map((row) => row.newPath)
    .filter((filePath) => isNetNewSourcePath(filePath))
    .sort();
  const addedScripts = addedPackageScripts(baseRef);
  const addedGateIds = addedToolingGateIds(baseRef);

  const requiresSrsRevisionUpdate =
    netNewSourceFiles.length > 0 || addedScripts.length > 0 || addedGateIds.length > 0;
  const rowDeltaPresent = srsChanged ? srsHasRowDelta(baseRef) : false;

  const violations: Array<{ id: string; detail: string }> = [];
  if (requiresSrsRevisionUpdate && !srsChanged) {
    violations.push({
      id: 'srs_same_revision_update_missing',
      detail:
        'Net-new functionality signals detected (new source/scripts/gates) but docs/workspace/SRS.md was not updated in the same revision.',
    });
  }
  if (requiresSrsRevisionUpdate && srsChanged && !rowDeltaPresent) {
    violations.push({
      id: 'srs_row_delta_missing',
      detail:
        'SRS changed but no SRS row delta line (+| V... |) was detected; add or update a concrete SRS row for the net-new functionality.',
    });
  }

  const payload = {
    ok: violations.length === 0,
    type: 'srs_same_revision_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: args.strict,
    base_ref: baseRef,
    summary: {
      changed_files: changedPaths.length,
      net_new_source_files: netNewSourceFiles.length,
      added_package_scripts: addedScripts.length,
      added_tooling_gate_ids: addedGateIds.length,
      requires_srs_revision_update: requiresSrsRevisionUpdate,
      srs_changed: srsChanged,
      srs_row_delta_present: rowDeltaPresent,
      violations: violations.length,
    },
    changed_files: changedPaths,
    net_new_source_files: netNewSourceFiles,
    added_package_scripts: addedScripts,
    added_tooling_gate_ids: addedGateIds,
    violations,
  };

  writeTextArtifact(path.resolve(ROOT, args.outMarkdown), toMarkdown(payload));
  process.exit(
    emitStructuredResult(payload, {
      outPath: path.resolve(ROOT, args.outJson),
      strict: args.strict,
      ok: payload.ok,
    }),
  );
}

main();
