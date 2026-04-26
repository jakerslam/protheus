#!/usr/bin/env node
/* eslint-disable no-console */
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { extname, resolve } from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_POLICY = 'client/runtime/config/shell_alpine_growth_policy.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_alpine_growth_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_ALPINE_GROWTH_GUARD_CURRENT.md';

type PatternConfig = {
  id: string;
  description?: string;
  regex: string;
};

type PolicyDoc = {
  path: string;
  must_include?: string[];
};

type PatternBaseline = {
  total: number;
  files: Record<string, number>;
};

type Policy = {
  version?: string;
  scan_roots?: string[];
  scan_extensions?: string[];
  ignore_path_contains?: string[];
  required_policy_docs?: PolicyDoc[];
  patterns?: PatternConfig[];
  baseline?: Record<string, PatternBaseline>;
};

type Args = {
  policyPath: string;
  strict: boolean;
  outJson: string;
  outMarkdown: string;
};

type PatternResult = {
  id: string;
  description: string;
  total: number;
  baseline_total: number;
  files_with_hits: number;
  files_over_baseline: number;
};

type Violation = {
  kind: string;
  pattern_id?: string;
  path?: string;
  baseline?: number | string;
  current?: number | string;
  detail: string;
};

function readArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  return {
    policyPath: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY, 400),
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
  };
}

function readJson<T>(path: string): T {
  return JSON.parse(readFileSync(resolve(ROOT, path), 'utf8')) as T;
}

function gitFiles(args: string[]): string[] {
  try {
    const output = execFileSync('git', args, { cwd: ROOT, encoding: 'utf8' });
    return output.split('\0').map((file) => file.trim()).filter(Boolean);
  } catch {
    return [];
  }
}

function shellFiles(policy: Policy): string[] {
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

function duplicateValues(values: string[]): string[] {
  const seen = new Set<string>();
  const dupes = new Set<string>();
  for (const value of values) {
    if (seen.has(value)) dupes.add(value);
    seen.add(value);
  }
  return [...dupes].sort();
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

function validatePolicy(policy: Policy): Violation[] {
  const violations: Violation[] = [];
  for (const root of policy.scan_roots ?? []) {
    if (!existsSync(resolve(ROOT, root))) {
      violations.push({
        kind: 'missing_scan_root',
        path: root,
        detail: 'Configured Alpine scan root does not exist.',
      });
    }
  }
  for (const duplicate of duplicateValues((policy.patterns ?? []).map((pattern) => pattern.id))) {
    violations.push({
      kind: 'duplicate_pattern_id',
      pattern_id: duplicate,
      detail: 'Pattern IDs must be unique.',
    });
  }
  for (const pattern of policy.patterns ?? []) {
    if (!compile(pattern)) {
      violations.push({
        kind: 'invalid_regex',
        pattern_id: pattern.id,
        detail: pattern.regex,
      });
    }
    if (!policy.baseline?.[pattern.id]) {
      violations.push({
        kind: 'missing_baseline',
        pattern_id: pattern.id,
        detail: 'Every Alpine detector must have an explicit legacy baseline.',
      });
    }
  }
  for (const doc of policy.required_policy_docs ?? []) {
    const abs = resolve(ROOT, doc.path);
    if (!existsSync(abs)) {
      violations.push({
        kind: 'missing_policy_doc',
        path: doc.path,
        detail: 'Required Shell Alpine policy document is missing.',
      });
      continue;
    }
    const source = readFileSync(abs, 'utf8');
    for (const token of doc.must_include ?? []) {
      if (!source.includes(token)) {
        violations.push({
          kind: 'policy_doc_missing_token',
          path: doc.path,
          current: token,
          detail: 'Required policy document token is absent.',
        });
      }
    }
  }
  return violations;
}

function scan(policy: Policy, files: string[]): { results: PatternResult[]; violations: Violation[] } {
  const violations: Violation[] = [];
  const results: PatternResult[] = [];
  for (const pattern of policy.patterns ?? []) {
    const regex = compile(pattern);
    if (!regex) continue;
    const baseline = policy.baseline?.[pattern.id] ?? { total: 0, files: {} };
    let total = 0;
    let filesWithHits = 0;
    let filesOverBaseline = 0;
    for (const file of files) {
      const count = countMatches(readFileSync(resolve(ROOT, file), 'utf8'), regex);
      if (count === 0) continue;
      total += count;
      filesWithHits += 1;
      const baselineCount = baseline.files[file] ?? 0;
      if (count > baselineCount) {
        filesOverBaseline += 1;
        violations.push({
          kind: 'file_alpine_growth',
          pattern_id: pattern.id,
          path: file,
          baseline: baselineCount,
          current: count,
          detail: 'File has more Alpine usage than the explicit legacy baseline.',
        });
      }
    }
    if (total > baseline.total) {
      violations.push({
        kind: 'total_alpine_growth',
        pattern_id: pattern.id,
        baseline: baseline.total,
        current: total,
        detail: 'Total Alpine usage exceeds the explicit legacy baseline.',
      });
    }
    results.push({
      id: pattern.id,
      description: pattern.description ?? '',
      total,
      baseline_total: baseline.total,
      files_with_hits: filesWithHits,
      files_over_baseline: filesOverBaseline,
    });
  }
  return { results, violations };
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell Alpine Growth Guard');
  lines.push('');
  lines.push(`- generated_at: ${payload.generated_at}`);
  lines.push(`- strict: ${payload.strict}`);
  lines.push(`- pass: ${payload.ok}`);
  lines.push(`- scanned_files: ${payload.summary.scanned_files}`);
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push('');
  lines.push('## Pattern Baseline');
  lines.push('| pattern | current | baseline | files with hits | files over baseline |');
  lines.push('| --- | ---: | ---: | ---: | ---: |');
  for (const row of payload.patterns) {
    lines.push(`| ${row.id} | ${row.total} | ${row.baseline_total} | ${row.files_with_hits} | ${row.files_over_baseline} |`);
  }
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) {
    lines.push('- none');
  } else {
    lines.push('| kind | pattern | path | baseline | current | detail |');
    lines.push('| --- | --- | --- | ---: | ---: | --- |');
    for (const row of payload.violations) {
      lines.push(`| ${row.kind} | ${row.pattern_id ?? ''} | ${row.path ?? ''} | ${row.baseline ?? ''} | ${row.current ?? ''} | ${row.detail} |`);
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function main(): void {
  const args = readArgs(process.argv.slice(2));
  const policy = readJson<Policy>(args.policyPath);
  const files = shellFiles(policy);
  const policyViolations = validatePolicy(policy);
  const scanResult = scan(policy, files);
  const violations = [...policyViolations, ...scanResult.violations];
  const ok = violations.length === 0;
  const payload = {
    ok,
    type: 'shell_alpine_growth_guard',
    generated_at: new Date().toISOString(),
    strict: args.strict,
    policy_path: args.policyPath,
    summary: {
      pass: ok,
      scanned_files: files.length,
      pattern_count: policy.patterns?.length ?? 0,
      violations: violations.length,
      files_over_baseline: scanResult.results.reduce((sum, row) => sum + row.files_over_baseline, 0),
    },
    patterns: scanResult.results,
    violations,
    artifact_paths: [args.outJson, args.outMarkdown],
  };
  writeTextArtifact(args.outMarkdown, markdown(payload));
  process.exitCode = emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok,
  });
}

main();
