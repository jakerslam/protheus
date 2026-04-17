#!/usr/bin/env node
/* eslint-disable no-console */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { extname, join, relative, resolve } from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const DEFAULT_POLICY_PATH = 'client/runtime/config/flaky_quarantine_allowlist.json';
const OUT_JSON = 'core/local/artifacts/flaky_quarantine_audit_current.json';
const OUT_MD = 'local/workspace/reports/FLAKY_QUARANTINE_AUDIT_CURRENT.md';

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { strict: false });
  return {
    strict: common.strict,
    policyPath: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY_PATH, 260),
    outJson: cleanText(readFlag(argv, 'out-json') || OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || OUT_MD, 400),
  };
}

function readJson(filePath: string) {
  return JSON.parse(readFileSync(resolve(filePath), 'utf8'));
}

function listFiles(root) {
  const files = [];
  function walk(dir) {
    if (!existsSync(dir)) return;
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) {
        walk(full);
        continue;
      }
      if (extname(entry.name) !== '.ts') continue;
      files.push(full);
    }
  }
  walk(resolve(root));
  return files.sort();
}

function normalized(path) {
  return path.replaceAll('\\', '/');
}

function findSkipRows(path) {
  const rows = [];
  const content = readFileSync(path, 'utf8');
  const lines = content.split(/\r?\n/);
  for (let idx = 0; idx < lines.length; idx += 1) {
    const line = lines[idx];
    if (/\b(test|it)\.skip\s*\(/.test(line) || /\b(test|it)\.todo\s*\(/.test(line)) {
      rows.push({
        line: idx + 1,
        text: line.trim().slice(0, 180),
      });
    }
  }
  return rows;
}

function toMarkdown(payload) {
  const lines = [];
  lines.push('# Flaky Quarantine Audit (Current)');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- pass: ${payload.summary.pass ? 'true' : 'false'}`);
  lines.push(`- strict: ${payload.summary.strict ? 'true' : 'false'}`);
  lines.push(`- scanned_files: ${payload.summary.scanned_files}`);
  lines.push(`- skip_occurrences: ${payload.summary.skip_occurrences}`);
  lines.push(`- allowlist_entries: ${payload.summary.allowlist_entries}`);
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push('');
  if (payload.violations.length > 0) {
    lines.push('## Violations');
    for (const row of payload.violations) {
      lines.push(`- ${row}`);
    }
    lines.push('');
  }
  if (payload.skip_rows.length > 0) {
    lines.push('## Skip/Todo Rows');
    lines.push('| File | Line | Snippet |');
    lines.push('| --- | ---: | --- |');
    for (const row of payload.skip_rows) {
      lines.push(`| ${row.path} | ${row.line} | ${row.text.replaceAll('|', '\\|')} |`);
    }
    lines.push('');
  }
  return `${lines.join('\n')}\n`;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const policy = readJson(args.policyPath);
  const scanRoot = resolve(policy.scan_root || 'tests/vitest');
  const allowlist = new Set(
    (policy.allowed_skip_files || [])
      .map((row) => normalized(String(row?.path || '').trim()))
      .filter(Boolean),
  );

  const files = listFiles(scanRoot);
  const skipRows = [];
  const violations = [];
  for (const fullPath of files) {
    const rel = normalized(relative(resolve('.'), fullPath));
    const hits = findSkipRows(fullPath);
    for (const hit of hits) {
      skipRows.push({ path: rel, line: hit.line, text: hit.text });
      if (!allowlist.has(rel)) {
        violations.push(`unallowlisted_skip_or_todo:${rel}:${hit.line}`);
      }
    }
  }

  for (const allow of allowlist) {
    const found = skipRows.some((row) => row.path === allow);
    if (!found) {
      violations.push(`stale_allowlist_entry:${allow}`);
    }
  }

  const payload = {
    ok: violations.length === 0,
    type: 'flaky_quarantine_audit',
    generated_at: new Date().toISOString(),
    revision: currentRevision(),
    policy_path: args.policyPath,
    summary: {
      strict: args.strict,
      scanned_files: files.length,
      skip_occurrences: skipRows.length,
      allowlist_entries: allowlist.size,
      violations: violations.length,
      pass: violations.length === 0,
    },
    artifact_paths: [args.outJson, args.outMarkdown],
    skip_rows: skipRows,
    violations,
  };

  writeTextArtifact(args.outMarkdown, toMarkdown(payload));
  process.exitCode = emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok: violations.length === 0,
  });
}

main();
