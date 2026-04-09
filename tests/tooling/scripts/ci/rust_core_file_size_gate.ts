#!/usr/bin/env node
/* eslint-disable no-console */
import { execSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

const DEFAULTS = {
  strict: false,
  policyPath: 'docs/workspace/rust_core_file_size_policy.json',
  outJson: 'core/local/artifacts/rust_core_file_size_gate_current.json',
  outMarkdown: 'local/workspace/reports/RUST_CORE_FILE_SIZE_GATE_CURRENT.md',
};

function parseArgs(argv) {
  const out = { ...DEFAULTS };
  for (const raw of argv) {
    const arg = String(raw || '').trim();
    if (!arg) continue;
    if (arg === '--strict' || arg === '--strict=1') {
      out.strict = true;
      continue;
    }
    if (arg.startsWith('--strict=')) {
      const value = arg.slice('--strict='.length).toLowerCase();
      out.strict = ['1', 'true', 'yes', 'on'].includes(value);
      continue;
    }
    if (arg.startsWith('--policy=')) {
      out.policyPath = arg.slice('--policy='.length).trim() || DEFAULTS.policyPath;
      continue;
    }
    if (arg.startsWith('--out-json=')) {
      out.outJson = arg.slice('--out-json='.length).trim() || DEFAULTS.outJson;
      continue;
    }
    if (arg.startsWith('--out-markdown=')) {
      out.outMarkdown =
        arg.slice('--out-markdown='.length).trim() || DEFAULTS.outMarkdown;
      continue;
    }
  }
  return out;
}

function ensureParent(path) {
  mkdirSync(dirname(resolve(path)), { recursive: true });
}

function readJson(path) {
  return JSON.parse(readFileSync(resolve(path), 'utf8'));
}

function listRustCoreFiles() {
  const output = execSync("rg --files core -g '*.rs'", { encoding: 'utf8' });
  return output
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
    .filter((line) => !isTestPath(line))
    .sort((a, b) => a.localeCompare(b));
}

function isTestPath(path) {
  return (
    /(^|\/)tests\//.test(path) ||
    /\.test\.(t|j)sx?$/.test(path) ||
    /(^|\/)__tests__(\/|$)/.test(path)
  );
}

function lineCount(path) {
  const content = readFileSync(resolve(path), 'utf8');
  return content.split(/\r?\n/).length;
}

function isExpired(dateIso, now) {
  const ts = Date.parse(String(dateIso || '').trim());
  if (!Number.isFinite(ts)) return true;
  return ts < now.getTime();
}

function toMarkdown(payload) {
  const lines = [];
  lines.push('# Rust Core File Size Gate (Current)');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Policy: ${payload.policy_path}`);
  lines.push(`Pass: ${payload.summary.pass ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- total_files: ${payload.summary.total_files}`);
  lines.push(`- max_lines: ${payload.summary.max_lines}`);
  lines.push(`- oversize_files: ${payload.summary.oversize_files}`);
  lines.push(`- exempt_files: ${payload.summary.exempt_files}`);
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push(`- strict: ${payload.summary.strict}`);
  lines.push('');
  if (payload.violations.length) {
    lines.push('## Violations');
    lines.push('| Path | Lines | Code | Detail |');
    lines.push('| --- | ---: | --- | --- |');
    for (const row of payload.violations) {
      lines.push(`| ${row.path} | ${row.lines} | ${row.code} | ${row.detail} |`);
    }
    lines.push('');
  }
  lines.push('## Oversize Inventory');
  lines.push('| Path | Lines | Status | Expires |');
  lines.push('| --- | ---: | --- | --- |');
  for (const row of payload.oversize_inventory) {
    lines.push(
      `| ${row.path} | ${row.lines} | ${row.status} | ${row.expires || ''} |`,
    );
  }
  return `${lines.join('\n')}\n`;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const now = new Date();
  const policy = readJson(args.policyPath);
  const maxLines = Number(policy?.max_lines || 500);
  const exceptionRows = Array.isArray(policy?.exceptions) ? policy.exceptions : [];
  const exceptionMap = new Map();
  for (const row of exceptionRows) {
    const path = String(row?.path || '').trim();
    if (!path) continue;
    exceptionMap.set(path, row);
  }

  const files = listRustCoreFiles();
  const oversizeInventory = [];
  const violations = [];
  let exemptCount = 0;

  for (const path of files) {
    const lines = lineCount(path);
    if (lines <= maxLines) continue;
    const exception = exceptionMap.get(path) || null;
    if (!exception) {
      oversizeInventory.push({ path, lines, status: 'violation_unlisted', expires: null });
      violations.push({
        path,
        lines,
        code: 'oversize_unlisted',
        detail: `file exceeds ${maxLines} lines without an exception entry`,
      });
      continue;
    }

    const owner = String(exception.owner || '').trim();
    const reason = String(exception.reason || '').trim();
    const expires = String(exception.expires || '').trim();
    if (!owner || !reason || !expires) {
      oversizeInventory.push({ path, lines, status: 'violation_metadata', expires: expires || null });
      violations.push({
        path,
        lines,
        code: 'exception_metadata_missing',
        detail: 'exception entry must include owner, reason, and expires',
      });
      continue;
    }

    if (isExpired(expires, now)) {
      oversizeInventory.push({ path, lines, status: 'violation_expired', expires });
      violations.push({
        path,
        lines,
        code: 'exception_expired',
        detail: `exception expired on ${expires}`,
      });
      continue;
    }

    oversizeInventory.push({ path, lines, status: 'exempt', expires });
    exemptCount += 1;
  }

  oversizeInventory.sort((a, b) => b.lines - a.lines || a.path.localeCompare(b.path));

  const payload = {
    ok: violations.length === 0,
    type: 'rust_core_file_size_gate',
    generated_at: now.toISOString(),
    policy_path: args.policyPath,
    summary: {
      strict: args.strict,
      pass: violations.length === 0,
      total_files: files.length,
      max_lines: maxLines,
      oversize_files: oversizeInventory.length,
      exempt_files: exemptCount,
      violations: violations.length,
    },
    violations,
    oversize_inventory: oversizeInventory,
  };

  ensureParent(args.outJson);
  ensureParent(args.outMarkdown);
  writeFileSync(resolve(args.outJson), `${JSON.stringify(payload, null, 2)}\n`);
  writeFileSync(resolve(args.outMarkdown), toMarkdown(payload));
  console.log(JSON.stringify(payload, null, 2));

  if (args.strict && violations.length > 0) {
    process.exitCode = 1;
  }
}

main();
