#!/usr/bin/env node
/* eslint-disable no-console */
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { extname, resolve } from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_POLICY = 'client/runtime/config/shell_alpine_growth_policy.json';
const DEFAULT_ROUTER = 'adapters/runtime/dashboard_asset_router.ts';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_alpine_retirement_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_ALPINE_RETIREMENT_GUARD_CURRENT.md';

type PatternConfig = { id: string; description?: string; regex: string };
type Policy = {
  scan_roots?: string[];
  scan_extensions?: string[];
  ignore_path_contains?: string[];
  patterns?: PatternConfig[];
};
type Args = {
  policyPath: string;
  routerPath: string;
  strict: boolean;
  outJson: string;
  outMarkdown: string;
};
type PatternResult = {
  id: string;
  description: string;
  total: number;
  files_with_hits: number;
  top_files: Array<{ path: string; count: number }>;
};
type Violation = {
  kind: string;
  pattern_id?: string;
  path?: string;
  current?: number | string;
  detail: string;
};

function args(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  return {
    policyPath: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY, 400),
    routerPath: cleanText(readFlag(argv, 'router') || DEFAULT_ROUTER, 400),
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
  };
}

function read(path: string): string {
  return readFileSync(resolve(ROOT, path), 'utf8');
}

function readJson<T>(path: string): T {
  return JSON.parse(read(path)) as T;
}

function gitFiles(args: string[]): string[] {
  try {
    const output = execFileSync('git', args, { cwd: ROOT, encoding: 'utf8' });
    return output.split('\0').map((file) => file.trim()).filter(Boolean);
  } catch {
    return [];
  }
}

function scanFiles(policy: Policy): string[] {
  const roots = policy.scan_roots ?? [];
  const extensions = new Set(policy.scan_extensions ?? []);
  const ignored = policy.ignore_path_contains ?? [];
  const files = new Set([...gitFiles(['ls-files', '-z']), ...gitFiles(['ls-files', '--others', '--exclude-standard', '-z'])]);
  return [...files].filter((file) => {
    const underRoot = roots.length === 0 || roots.some((root) => file === root || file.startsWith(`${root}/`));
    const hasExtension = extensions.size === 0 || extensions.has(extname(file));
    const isIgnored = ignored.some((needle) => needle && file.includes(needle));
    return underRoot && hasExtension && !isIgnored && existsSync(resolve(ROOT, file));
  }).sort();
}

function compile(pattern: PatternConfig): RegExp | null {
  try {
    return new RegExp(pattern.regex, 'g');
  } catch {
    return null;
  }
}

function countMatches(source: string, regex: RegExp): number {
  regex.lastIndex = 0;
  let count = 0;
  let match = regex.exec(source);
  while (match) {
    count += 1;
    if (match[0] === '') regex.lastIndex += 1;
    match = regex.exec(source);
  }
  return count;
}

function scan(policy: Policy, files: string[]): { results: PatternResult[]; violations: Violation[] } {
  const results: PatternResult[] = [];
  const violations: Violation[] = [];
  for (const pattern of policy.patterns ?? []) {
    const regex = compile(pattern);
    if (!regex) {
      violations.push({
        kind: 'invalid_alpine_retirement_regex',
        pattern_id: pattern.id,
        detail: 'Alpine retirement detector regex is invalid.',
      });
      continue;
    }
    const counts: Array<{ path: string; count: number }> = [];
    for (const file of files) {
      const count = countMatches(read(file), regex);
      if (count > 0) counts.push({ path: file, count });
    }
    const total = counts.reduce((sum, row) => sum + row.count, 0);
    if (total > 0) {
      violations.push({
        kind: 'live_alpine_usage',
        pattern_id: pattern.id,
        current: total,
        detail: 'Alpine retirement requires zero live Alpine usage in shell scan roots.',
      });
    }
    results.push({
      id: pattern.id,
      description: pattern.description ?? '',
      total,
      files_with_hits: counts.length,
      top_files: counts.sort((a, b) => b.count - a.count || a.path.localeCompare(b.path)).slice(0, 8),
    });
  }
  return { results, violations };
}

function routerViolations(routerPath: string): Violation[] {
  if (!existsSync(resolve(ROOT, routerPath))) {
    return [{ kind: 'missing_dashboard_router', path: routerPath, detail: 'Dashboard asset router must exist for Alpine retirement proof.' }];
  }
  const router = read(routerPath);
  const checks = [
    { token: "vendor/alpine.min", kind: 'alpine_vendor_loader_present', detail: 'Dashboard router still loads the Alpine vendor bundle.' },
  ];
  return checks
    .filter((check) => router.includes(check.token))
    .map((check) => ({ kind: check.kind, path: routerPath, current: check.token, detail: check.detail }));
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell Alpine Retirement Guard');
  lines.push('');
  lines.push(`- generated_at: ${payload.generated_at}`);
  lines.push(`- pass: ${payload.ok}`);
  lines.push(`- scanned_files: ${payload.summary.scanned_files}`);
  lines.push(`- total_live_alpine_hits: ${payload.summary.total_live_alpine_hits}`);
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push('');
  lines.push('## Live Alpine Usage');
  lines.push('| pattern | hits | files | top files |');
  lines.push('| --- | ---: | ---: | --- |');
  for (const row of payload.patterns) {
    const top = row.top_files.map((file: any) => `${file.path} (${file.count})`).join('<br>');
    lines.push(`| ${row.id} | ${row.total} | ${row.files_with_hits} | ${top || '-'} |`);
  }
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) lines.push('- none');
  for (const violation of payload.violations) {
    lines.push(`- ${violation.kind}: ${violation.pattern_id || violation.path || 'shell'} (${violation.current ?? ''}) ${violation.detail}`);
  }
  return `${lines.join('\n')}\n`;
}

async function run(argv = process.argv.slice(2)) {
  const parsed = args(argv);
  const missing: Violation[] = [];
  if (!existsSync(resolve(ROOT, parsed.policyPath))) {
    missing.push({ kind: 'missing_alpine_growth_policy', path: parsed.policyPath, detail: 'Alpine retirement guard reuses the growth policy detector set.' });
  }
  const policy = missing.length ? { patterns: [] } : readJson<Policy>(parsed.policyPath);
  const files = missing.length ? [] : scanFiles(policy);
  const scanned = scan(policy, files);
  const violations = [...missing, ...scanned.violations, ...routerViolations(parsed.routerPath)];
  const totalLiveAlpineHits = scanned.results.reduce((sum, row) => sum + row.total, 0);
  const payload = {
    ok: violations.length === 0,
    type: 'shell_alpine_retirement_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: parsed.strict,
    summary: {
      pass: violations.length === 0,
      scanned_files: files.length,
      patterns: scanned.results.length,
      total_live_alpine_hits: totalLiveAlpineHits,
      violations: violations.length,
    },
    patterns: scanned.results,
    violations,
  };
  writeTextArtifact(parsed.outMarkdown, markdown(payload));
  emitStructuredResult(payload, { ok: payload.ok, outPath: parsed.outJson });
  if (!payload.ok && parsed.strict) process.exitCode = 1;
}

run().catch((error) => {
  const payload = { ok: false, type: 'shell_alpine_retirement_guard', error: error instanceof Error ? error.message : String(error) };
  emitStructuredResult(payload, { ok: false, outPath: DEFAULT_OUT_JSON });
  process.exitCode = 1;
});
