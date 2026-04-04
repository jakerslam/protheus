#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

type CommitRow = {
  sha: string;
  subject: string;
  body: string;
};

type GateReport = {
  ok: boolean;
  type: 'conventional_commit_gate';
  strict: boolean;
  range: string;
  scanned: number;
  invalid_count: number;
  invalid: Array<{ sha: string; subject: string; reason: string }>;
  accepted_examples: string[];
};

function cleanText(value: unknown, maxLen = 6000): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function parseBool(value: string | undefined, fallback = false): boolean {
  const raw = cleanText(value ?? '', 32).toLowerCase();
  if (!raw) return fallback;
  return raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on';
}

function parseArgs(argv: string[]) {
  const out = {
    from: '',
    to: 'HEAD',
    strict: false,
    outPath: '',
  };
  for (const tokenRaw of argv) {
    const token = cleanText(tokenRaw, 400);
    if (!token) continue;
    if (token.startsWith('--from=')) out.from = cleanText(token.slice(7), 120);
    else if (token.startsWith('--to=')) out.to = cleanText(token.slice(5), 120) || 'HEAD';
    else if (token.startsWith('--strict=')) out.strict = parseBool(token.slice(9), false);
    else if (token.startsWith('--out=')) out.outPath = cleanText(token.slice(6), 400);
  }
  return out;
}

function runGit(args: string[], cwd: string): string {
  return String(
    execFileSync('git', args, {
      cwd,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    }) || ''
  );
}

function readCommits(root: string, rangeExpr: string): CommitRow[] {
  const format = '%H%x1f%s%x1f%b%x1e';
  const output = runGit(['log', '--format=' + format, rangeExpr], root);
  return output
    .split('\x1e')
    .map((chunk) => chunk.trim())
    .filter(Boolean)
    .map((chunk) => {
      const parts = chunk.split('\x1f');
      return {
        sha: cleanText(parts[0] ?? '', 80),
        subject: cleanText(parts[1] ?? '', 400),
        body: cleanText(parts[2] ?? '', 4000),
      };
    })
    .filter((row) => row.sha && row.subject);
}

function isConventionalSubject(subject: string): boolean {
  const s = cleanText(subject, 300);
  if (!s) return false;
  if (/^Merge\b/.test(s)) return true;
  if (/^Revert\b/.test(s)) return true;
  if (/^chore\(release\):\s*v\d+\.\d+\.\d+(?:-[a-z0-9.-]+)?$/i.test(s)) return true;
  return /^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([^)]+\))?(!)?: .+/i.test(
    s
  );
}

function reasonForInvalid(subject: string): string {
  const s = cleanText(subject, 300);
  if (!s) return 'empty_subject';
  if (!/^[a-z]/i.test(s)) return 'subject_missing_type_prefix';
  if (!s.includes(':')) return 'missing_colon_delimiter';
  return 'non_conventional_subject';
}

function main() {
  const root = path.resolve(__dirname, '../../../..');
  const args = parseArgs(process.argv.slice(2));
  const range = args.from ? `${args.from}..${args.to}` : `${args.to}~30..${args.to}`;
  let rows: CommitRow[] = [];
  try {
    rows = readCommits(root, range);
  } catch {
    rows = [];
  }
  const invalid = rows
    .filter((row) => !isConventionalSubject(row.subject))
    .map((row) => ({
      sha: row.sha,
      subject: row.subject,
      reason: reasonForInvalid(row.subject),
    }));
  const report: GateReport = {
    ok: invalid.length === 0,
    type: 'conventional_commit_gate',
    strict: args.strict,
    range,
    scanned: rows.length,
    invalid_count: invalid.length,
    invalid,
    accepted_examples: [
      'feat(router): discover local ollama models',
      'fix(installer): verify checksum manifest before install',
      'chore(release): v0.4.0-alpha',
    ],
  };
  if (args.outPath) {
    const outPath = path.resolve(root, args.outPath);
    fs.mkdirSync(path.dirname(outPath), { recursive: true });
    fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  }
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (args.strict && !report.ok) process.exitCode = 1;
}

main();
