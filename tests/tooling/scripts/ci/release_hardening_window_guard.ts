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

function gitDiffNames(baseRef: string): string[] {
  const compare = baseRef ? `${baseRef}...HEAD` : 'HEAD~1..HEAD';
  const out = spawnSync('git', ['diff', '--name-only', '--diff-filter=ACMR', compare], {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (out.status !== 0) return [];
  return String(out.stdout || '')
    .split('\n')
    .map((row) => row.trim())
    .filter(Boolean);
}

function buildReport(baseRefOverride = '') {
  const policy = JSON.parse(fs.readFileSync(POLICY_PATH, 'utf8'));
  const active = parseBool(process.env[String(policy.activation_env || 'INFRING_RELEASE_HARDENING_WINDOW')], false);
  const baseRef = baseRefOverride || clean(policy.default_base_ref, 120);
  const changed = gitDiffNames(baseRef);
  const blockedPrefixes = Array.isArray(policy.blocked_prefixes) ? policy.blocked_prefixes : [];
  const violations = active
    ? changed.filter((filePath) => blockedPrefixes.some((prefix) => filePath.startsWith(String(prefix))))
    : [];
  return {
    ok: !active || violations.length === 0,
    type: 'release_hardening_window_guard',
    generated_at: new Date().toISOString(),
    active,
    base_ref: baseRef,
    changed_files: changed,
    blocked_prefixes: blockedPrefixes,
    violations,
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
