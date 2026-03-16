#!/usr/bin/env node
/* eslint-disable no-console */
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, extname, join, relative, resolve } from 'node:path';

const DEFAULT_POLICY_PATH = 'client/runtime/config/flaky_quarantine_allowlist.json';
const OUT_JSON = 'core/local/artifacts/flaky_quarantine_audit_current.json';
const OUT_MD = 'local/workspace/reports/FLAKY_QUARANTINE_AUDIT_CURRENT.md';

function parseArgs(argv) {
  const out = { strict: false, policyPath: DEFAULT_POLICY_PATH };
  for (const raw of argv) {
    const arg = String(raw ?? '').trim();
    if (!arg) continue;
    if (arg === '--strict' || arg === '--strict=1') {
      out.strict = true;
      continue;
    }
    if (arg.startsWith('--strict=')) {
      out.strict = ['1', 'true', 'yes', 'on'].includes(arg.slice('--strict='.length).toLowerCase());
      continue;
    }
    if (arg.startsWith('--policy=')) {
      out.policyPath = arg.slice('--policy='.length).trim() || DEFAULT_POLICY_PATH;
      continue;
    }
  }
  return out;
}

function readJson(path) {
  return JSON.parse(readFileSync(resolve(path), 'utf8'));
}

function ensureParent(path) {
  mkdirSync(dirname(resolve(path)), { recursive: true });
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
    ok: true,
    type: 'flaky_quarantine_audit',
    generated_at: new Date().toISOString(),
    policy_path: args.policyPath,
    summary: {
      strict: args.strict,
      scanned_files: files.length,
      skip_occurrences: skipRows.length,
      allowlist_entries: allowlist.size,
      violations: violations.length,
      pass: violations.length === 0,
    },
    skip_rows: skipRows,
    violations,
  };

  ensureParent(OUT_JSON);
  ensureParent(OUT_MD);
  writeFileSync(resolve(OUT_JSON), `${JSON.stringify(payload, null, 2)}\n`);
  writeFileSync(resolve(OUT_MD), toMarkdown(payload));

  if (args.strict && violations.length > 0) {
    console.error(
      JSON.stringify(
        {
          ok: false,
          type: payload.type,
          out_json: OUT_JSON,
          summary: payload.summary,
          violations,
        },
        null,
        2,
      ),
    );
    process.exit(1);
  }

  console.log(
    JSON.stringify(
      {
        ok: true,
        type: payload.type,
        out_json: OUT_JSON,
        out_markdown: OUT_MD,
        summary: payload.summary,
      },
      null,
      2,
    ),
  );
}

main();
