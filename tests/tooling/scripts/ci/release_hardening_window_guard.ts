#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const ROOT = process.cwd();
const POLICY_PATH = path.join(ROOT, 'client/runtime/config/release_hardening_window_policy.json');
const DEFAULT_OUT = path.join(ROOT, 'core/local/artifacts/release_hardening_window_guard_current.json');

function clean(value: unknown, max = 240): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function parseBool(raw: string | undefined, fallback = false): boolean {
  const value = clean(raw, 24).toLowerCase();
  if (!value) return fallback;
  return value === '1' || value === 'true' || value === 'yes' || value === 'on';
}

function parseArgs(argv: string[]) {
  const parsed = {
    strict: false,
    out: DEFAULT_OUT,
    baseRef: '',
  };
  for (const tokenRaw of argv) {
    const token = clean(tokenRaw, 400);
    if (!token) continue;
    if (token.startsWith('--strict=')) parsed.strict = parseBool(token.slice(9), false);
    else if (token.startsWith('--out=')) parsed.out = path.resolve(ROOT, clean(token.slice(6), 400));
    else if (token.startsWith('--base-ref=')) parsed.baseRef = clean(token.slice(11), 200);
  }
  return parsed;
}

function parseJsonMaybe(raw: string): any {
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function gitDiffRows(baseRef: string): Array<{ status: string; file: string }> {
  const compare = baseRef ? `${baseRef}...HEAD` : 'HEAD~1..HEAD';
  const out = spawnSync('git', ['diff', '--name-status', '--diff-filter=ACMR', compare], {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (out.status !== 0) return [];
  return String(out.stdout || '')
    .split('\n')
    .map((row) => row.trim())
    .filter(Boolean)
    .map((row) => row.split('\t'))
    .map((parts) => ({
      status: clean(parts[0], 16),
      file: clean(parts[parts.length - 1], 400),
    }))
    .filter((row) => row.file);
}

function gitShowText(ref: string, relPath: string): string {
  const out = spawnSync('git', ['show', `${ref}:${relPath}`], {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore'],
  });
  return out.status === 0 ? String(out.stdout || '') : '';
}

function packageScriptKeysFromText(raw: string): string[] {
  const parsed = parseJsonMaybe(raw);
  const scripts = parsed && typeof parsed === 'object' ? (parsed as any).scripts : null;
  return scripts && typeof scripts === 'object' ? Object.keys(scripts).sort() : [];
}

function gateKeysFromText(raw: string): string[] {
  const parsed = parseJsonMaybe(raw);
  const gates = parsed && typeof parsed === 'object' ? (parsed as any).gates : null;
  return gates && typeof gates === 'object' ? Object.keys(gates).sort() : [];
}

function buildReport(baseRefOverride = '') {
  const policy = JSON.parse(fs.readFileSync(POLICY_PATH, 'utf8'));
  const active = parseBool(process.env[String(policy.activation_env || 'INFRING_RELEASE_HARDENING_WINDOW')], false);
  const baseRef = baseRefOverride || clean(policy.default_base_ref, 120);
  const changedRows = gitDiffRows(baseRef);
  const changed = changedRows.map((row) => row.file);
  const blockedPrefixes = Array.isArray(policy.blocked_prefixes) ? policy.blocked_prefixes : [];
  const blockedAddedFilePrefixes = Array.isArray(policy.blocked_added_file_prefixes)
    ? policy.blocked_added_file_prefixes
    : [];
  const allowedNewPackageScripts = Array.isArray(policy.allowed_new_package_scripts)
    ? policy.allowed_new_package_scripts.map((value: unknown) => clean(value, 160)).filter(Boolean)
    : [];
  const allowedNewToolingGateIds = Array.isArray(policy.allowed_new_tooling_gate_ids)
    ? policy.allowed_new_tooling_gate_ids.map((value: unknown) => clean(value, 160)).filter(Boolean)
    : [];
  const currentPackageJson = fs.existsSync(path.join(ROOT, 'package.json'))
    ? fs.readFileSync(path.join(ROOT, 'package.json'), 'utf8')
    : '{}';
  const basePackageJson = gitShowText(baseRef, 'package.json');
  const currentGateRegistry = fs.existsSync(path.join(ROOT, 'tests/tooling/config/tooling_gate_registry.json'))
    ? fs.readFileSync(path.join(ROOT, 'tests/tooling/config/tooling_gate_registry.json'), 'utf8')
    : '{}';
  const baseGateRegistry = gitShowText(baseRef, 'tests/tooling/config/tooling_gate_registry.json');
  const addedFiles = changedRows
    .filter((row) => ['A', 'C'].includes(row.status[0] || '') || row.status.startsWith('R'))
    .map((row) => row.file);
  const basePackageScriptKeys = packageScriptKeysFromText(basePackageJson);
  const currentPackageScriptKeys = packageScriptKeysFromText(currentPackageJson);
  const baseGateKeys = gateKeysFromText(baseGateRegistry);
  const currentGateKeys = gateKeysFromText(currentGateRegistry);
  const violations = active
    ? changed.filter((filePath) => blockedPrefixes.some((prefix) => filePath.startsWith(String(prefix))))
    : [];
  const addedFileViolations = active
    ? addedFiles.filter((filePath) =>
        blockedAddedFilePrefixes.some((prefix) => filePath.startsWith(String(prefix))),
      )
    : [];
  const newPackageScriptViolations =
    active && policy.block_new_package_scripts === true
      ? currentPackageScriptKeys.filter(
          (key) => !basePackageScriptKeys.includes(key) && !allowedNewPackageScripts.includes(key),
        )
      : [];
  const newToolingGateViolations =
    active && policy.block_new_tooling_gate_ids === true
      ? currentGateKeys.filter(
          (key) => !baseGateKeys.includes(key) && !allowedNewToolingGateIds.includes(key),
        )
      : [];
  return {
    ok:
      !active ||
      (violations.length === 0 &&
        addedFileViolations.length === 0 &&
        newPackageScriptViolations.length === 0 &&
        newToolingGateViolations.length === 0),
    type: 'release_hardening_window_guard',
    generated_at: new Date().toISOString(),
    active,
    base_ref: baseRef,
    changed_files: changedRows,
    blocked_prefixes: blockedPrefixes,
    blocked_added_file_prefixes: blockedAddedFilePrefixes,
    allowed_new_package_scripts: allowedNewPackageScripts,
    allowed_new_tooling_gate_ids: allowedNewToolingGateIds,
    violations,
    added_file_violations: addedFileViolations,
    new_package_script_violations: newPackageScriptViolations,
    new_tooling_gate_violations: newToolingGateViolations,
  };
}

function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const report = buildReport(args.baseRef);
  fs.mkdirSync(path.dirname(args.out), { recursive: true });
  fs.writeFileSync(args.out, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  process.stdout.write(`${JSON.stringify(report)}\n`);
  if (args.strict && report.ok !== true) return 1;
  return 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run, buildReport };
