#!/usr/bin/env node
/* eslint-disable no-console */
import { execSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

const DEFAULTS = {
  strict: false,
  policyPath: 'docs/workspace/repo_file_size_policy.json',
  outJson: 'core/local/artifacts/repo_file_size_gate_current.json',
  outMarkdown: 'local/workspace/reports/REPO_FILE_SIZE_GATE_CURRENT.md',
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
      out.outMarkdown = arg.slice('--out-markdown='.length).trim() || DEFAULTS.outMarkdown;
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

function shellQuote(value) {
  return `'${String(value).replace(/'/g, `'\\''`)}'`;
}

function scopeRoots(policy) {
  const raw = Array.isArray(policy?.scope) ? policy.scope : [];
  const normalized = raw
    .map((row) => String(row || '').trim())
    .filter(Boolean)
    .map((row) => row.replace(/\/\*\*$/, '').replace(/\/\*$/, '').replace(/\/+$/, ''))
    .filter(Boolean);
  const unique = [...new Set(normalized)];
  return unique.length > 0 ? unique : ['core', 'client'];
}

function listFiles(policy) {
  const roots = scopeRoots(policy);
  const commands = [
    `rg --files ${roots.map(shellQuote).join(' ')}`,
    `find ${roots.map(shellQuote).join(' ')} -type f`,
  ];
  for (const command of commands) {
    try {
      const output = execSync(command, {
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
      });
      const files = output
        .split('\n')
        .map((line) => line.trim())
        .filter(Boolean)
        .map((line) => (line.startsWith('./') ? line.slice(2) : line))
        .sort((a, b) => a.localeCompare(b));
      if (files.length > 0) {
        return files;
      }
    } catch {
      continue;
    }
  }
  return [];
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

function capFor(path, policy) {
  const ext = (path.match(/\.([^.]+)$/) || [])[1] || '';
  const uiSourceExt = new Set(['ts', 'tsx', 'js', 'jsx', 'css', 'html']);
  const tsJsExt = new Set(['ts', 'tsx', 'js', 'jsx']);
  if (path.startsWith('client/runtime/systems/ui/') && uiSourceExt.has(ext)) {
    return Number(policy?.caps?.ui_source || 500);
  }
  if (path.startsWith('core/') && ext === 'rs') {
    return Number(policy?.caps?.core_rust || 500);
  }
  if (tsJsExt.has(ext)) {
    return Number(policy?.caps?.other_ts_js || 1200);
  }
  if (ext === 'rs') {
    return Number(policy?.caps?.other_rust || 1000);
  }
  return null;
}

function isExpired(dateIso, now) {
  const ts = Date.parse(String(dateIso || '').trim());
  if (!Number.isFinite(ts)) return true;
  return ts < now.getTime();
}

function toMarkdown(payload) {
  const lines = [];
  lines.push('# Repo File Size Gate (Current)');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Policy: ${payload.policy_path}`);
  lines.push(`Pass: ${payload.summary.pass ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- total_scanned: ${payload.summary.total_scanned}`);
  lines.push(`- tests_exempt: ${payload.summary.tests_exempt}`);
  lines.push(`- oversized: ${payload.summary.oversized}`);
  lines.push(`- exempted: ${payload.summary.exempted}`);
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push(`- strict: ${payload.summary.strict}`);
  lines.push('');
  if (payload.violations.length) {
    lines.push('## Violations');
    lines.push('| Path | Lines | Cap | Code | Detail |');
    lines.push('| --- | ---: | ---: | --- | --- |');
    for (const row of payload.violations) {
      lines.push(
        `| ${row.path} | ${row.lines} | ${row.cap} | ${row.code} | ${row.detail} |`,
      );
    }
    lines.push('');
  }
  lines.push('## Oversized Inventory');
  lines.push('| Path | Lines | Cap | Status | Expires |');
  lines.push('| --- | ---: | ---: | --- | --- |');
  for (const row of payload.oversized_inventory) {
    lines.push(
      `| ${row.path} | ${row.lines} | ${row.cap} | ${row.status} | ${row.expires || ''} |`,
    );
  }
  return `${lines.join('\n')}\n`;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const now = new Date();
  const policy = readJson(args.policyPath);
  const files = listFiles(policy);
  const exceptionRows = Array.isArray(policy?.exceptions) ? policy.exceptions : [];
  const exceptionCountCeiling = Number(policy?.exception_count_ceiling || 0);
  const exceptionMap = new Map();
  for (const row of exceptionRows) {
    const path = String(row?.path || '').trim();
    if (!path) continue;
    exceptionMap.set(path, row);
  }

  const oversizedInventory = [];
  const violations = [];
  let scanned = 0;
  let testsExempt = 0;
  let exempted = 0;

  if (Number.isFinite(exceptionCountCeiling) && exceptionCountCeiling > 0 && exceptionRows.length > exceptionCountCeiling) {
    violations.push({
      path: '(policy)',
      lines: exceptionRows.length,
      cap: exceptionCountCeiling,
      code: 'exception_count_ceiling_exceeded',
      detail: `exceptions=${exceptionRows.length}; ceiling=${exceptionCountCeiling}`,
    });
  }

  for (const path of files) {
    if (isTestPath(path)) {
      testsExempt += 1;
      continue;
    }
    const cap = capFor(path, policy);
    if (!Number.isFinite(cap)) continue;
    scanned += 1;
    const lines = lineCount(path);
    if (lines <= cap) continue;

    const exception = exceptionMap.get(path) || null;
    if (!exception) {
      oversizedInventory.push({ path, lines, cap, status: 'violation_unlisted', expires: null });
      violations.push({
        path,
        lines,
        cap,
        code: 'oversize_unlisted',
        detail: 'missing exception entry',
      });
      continue;
    }

    const owner = String(exception.owner || '').trim();
    const reason = String(exception.reason || '').trim();
    const expires = String(exception.expires || '').trim();
    if (!owner || !reason || !expires) {
      oversizedInventory.push({ path, lines, cap, status: 'violation_metadata', expires: expires || null });
      violations.push({
        path,
        lines,
        cap,
        code: 'exception_metadata_missing',
        detail: 'exception requires owner, reason, expires',
      });
      continue;
    }

    if (isExpired(expires, now)) {
      oversizedInventory.push({ path, lines, cap, status: 'violation_expired', expires });
      violations.push({
        path,
        lines,
        cap,
        code: 'exception_expired',
        detail: `expired on ${expires}`,
      });
      continue;
    }

    oversizedInventory.push({ path, lines, cap, status: 'exempt', expires });
    exempted += 1;
  }

  oversizedInventory.sort((a, b) => b.lines - a.lines || a.path.localeCompare(b.path));
  const payload = {
    ok: violations.length === 0,
    type: 'repo_file_size_gate',
    generated_at: now.toISOString(),
    policy_path: args.policyPath,
    summary: {
      strict: args.strict,
      pass: violations.length === 0,
      total_scanned: scanned,
      tests_exempt: testsExempt,
      exception_count: exceptionRows.length,
      exception_count_ceiling: Number.isFinite(exceptionCountCeiling) && exceptionCountCeiling > 0 ? exceptionCountCeiling : null,
      oversized: oversizedInventory.length,
      exempted,
      violations: violations.length,
    },
    violations,
    oversized_inventory: oversizedInventory,
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
