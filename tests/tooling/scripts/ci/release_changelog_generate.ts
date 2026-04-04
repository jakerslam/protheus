#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

type CommitRow = {
  sha: string;
  subject: string;
  body: string;
};

function cleanText(value: unknown, maxLen = 6000): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function parseArgs(argv: string[]) {
  const out = {
    previousTag: '',
    nextTag: '',
    channel: 'stable',
    outPath: 'client/runtime/local/state/release/CHANGELOG.auto.md',
  };
  for (const tokenRaw of argv) {
    const token = cleanText(tokenRaw, 400);
    if (!token) continue;
    if (token.startsWith('--previous-tag=')) out.previousTag = cleanText(token.slice(15), 160);
    else if (token.startsWith('--next-tag=')) out.nextTag = cleanText(token.slice(11), 160);
    else if (token.startsWith('--channel=')) out.channel = cleanText(token.slice(10), 40).toLowerCase();
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
    .filter((row) => row.sha && row.subject)
    .filter((row) => !/^chore\(release\):/i.test(row.subject));
}

function isBreaking(row: CommitRow): boolean {
  if (/^[a-z]+(\([^)]+\))?!:/i.test(row.subject)) return true;
  return /(^|\n)\s*BREAKING[\s_-]CHANGE\s*:/i.test(cleanText(row.body, 4000));
}

function classify(row: CommitRow): 'breaking' | 'feat' | 'fix' | 'other' {
  if (isBreaking(row)) return 'breaking';
  if (/^feat(\([^)]+\))?:/i.test(row.subject)) return 'feat';
  if (/^fix(\([^)]+\))?:/i.test(row.subject)) return 'fix';
  return 'other';
}

function markdownSection(title: string, rows: CommitRow[]): string {
  if (!rows.length) return '';
  const lines = [`## ${title}`];
  for (const row of rows) {
    const sha = row.sha.slice(0, 8);
    lines.push(`- ${row.subject} (\`${sha}\`)`);
  }
  lines.push('');
  return lines.join('\n');
}

function main() {
  const root = path.resolve(__dirname, '../../../..');
  const args = parseArgs(process.argv.slice(2));
  const range = args.previousTag ? `${args.previousTag}..HEAD` : 'HEAD';
  const commits = readCommits(root, range);
  const groups = {
    breaking: commits.filter((row) => classify(row) === 'breaking'),
    feat: commits.filter((row) => classify(row) === 'feat'),
    fix: commits.filter((row) => classify(row) === 'fix'),
    other: commits.filter((row) => classify(row) === 'other'),
  };

  const now = new Date().toISOString().slice(0, 10);
  const tagLabel = args.nextTag || 'unreleased';
  const header = [
    `# ${tagLabel}`,
    '',
    `- Date: ${now}`,
    `- Channel: ${args.channel || 'stable'}`,
    `- Commits: ${commits.length}`,
    '',
  ].join('\n');
  const body = [
    markdownSection('Breaking Changes', groups.breaking),
    markdownSection('Features', groups.feat),
    markdownSection('Fixes', groups.fix),
    markdownSection('Other Changes', groups.other),
  ]
    .filter(Boolean)
    .join('\n');
  const out = `${header}${body || '## Changes\n- No releasable commits in this range.\n'}`;
  const outPath = path.resolve(root, args.outPath);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${out}\n`, 'utf8');
  process.stdout.write(
    `${JSON.stringify(
      {
        ok: true,
        type: 'release_changelog_generate',
        out_path: path.relative(root, outPath),
        previous_tag: args.previousTag || 'none',
        next_tag: args.nextTag || 'none',
        channel: args.channel,
        commit_count: commits.length,
      },
      null,
      2
    )}\n`
  );
}

main();
