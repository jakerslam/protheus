#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const ROOT = path.resolve(__dirname, '..', '..');
const DEFAULT_POLICY_PATH = path.join(ROOT, 'config', 'runtime_retention_policy.json');

function nowIso() {
  return new Date().toISOString();
}

function cleanText(v: unknown, maxLen = 260) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function parseArgs(argv: string[]) {
  const out: Record<string, any> = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const token = String(argv[i] || '').trim();
    if (!token.startsWith('--')) {
      out._.push(token);
      continue;
    }
    const eq = token.indexOf('=');
    if (eq >= 0) {
      out[token.slice(2, eq)] = token.slice(eq + 1);
      continue;
    }
    const key = token.slice(2);
    const next = argv[i + 1];
    if (next != null && !String(next).startsWith('--')) {
      out[key] = String(next);
      i += 1;
      continue;
    }
    out[key] = '1';
  }
  return out;
}

function toBool(v: unknown, fallback = false) {
  const raw = cleanText(v, 20).toLowerCase();
  if (!raw) return fallback;
  if (['1', 'true', 'yes', 'on'].includes(raw)) return true;
  if (['0', 'false', 'no', 'off'].includes(raw)) return false;
  return fallback;
}

function toInt(v: unknown, fallback: number, lo: number, hi: number) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  return Math.max(lo, Math.min(hi, Math.floor(n)));
}

function readJson(filePath: string, fallback: any = null) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function writeJson(filePath: string, payload: any) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function appendJsonl(filePath: string, payload: any) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.appendFileSync(filePath, `${JSON.stringify(payload)}\n`, 'utf8');
}

function resolvePath(rawPath: string) {
  const p = cleanText(rawPath, 800);
  if (!p) return '';
  return path.isAbsolute(p) ? p : path.join(ROOT, p);
}

function deterministicHash(payload: any) {
  return crypto.createHash('sha256').update(JSON.stringify(payload), 'utf8').digest('hex');
}

function pruneJsonl(targetPath: string, keepLines: number, apply = false) {
  if (!targetPath || !fs.existsSync(targetPath)) {
    return {
      path: targetPath,
      exists: false,
      keep_lines: keepLines,
      total_lines: 0,
      removed_lines: 0,
      applied: false
    };
  }
  const raw = String(fs.readFileSync(targetPath, 'utf8') || '');
  const lines = raw.split('\n').map((line) => line.trimEnd()).filter((line) => line.trim().length > 0);
  const total = lines.length;
  const keep = Math.max(1, keepLines);
  const removed = total > keep ? total - keep : 0;
  const keptLines = removed > 0 ? lines.slice(total - keep) : lines;
  if (apply && removed > 0) {
    fs.writeFileSync(targetPath, `${keptLines.join('\n')}\n`, 'utf8');
  }
  return {
    path: targetPath,
    exists: true,
    keep_lines: keep,
    total_lines: total,
    removed_lines: removed,
    applied: apply && removed > 0
  };
}

function pruneDirectory(targetPath: string, maxFiles: number, apply = false) {
  if (!targetPath || !fs.existsSync(targetPath)) {
    return {
      path: targetPath,
      exists: false,
      max_files: maxFiles,
      total_files: 0,
      removed_files: 0,
      applied: false
    };
  }
  const rows = fs.readdirSync(targetPath)
    .map((name: string) => {
      const abs = path.join(targetPath, name);
      try {
        const st = fs.statSync(abs);
        if (!st.isFile()) return null;
        return {
          name,
          abs,
          mtime_ms: Number(st.mtimeMs || 0)
        };
      } catch {
        return null;
      }
    })
    .filter(Boolean) as Array<{name: string, abs: string, mtime_ms: number}>;
  rows.sort((a, b) => a.mtime_ms - b.mtime_ms);
  const total = rows.length;
  const keep = Math.max(1, maxFiles);
  const removeRows = total > keep ? rows.slice(0, total - keep) : [];
  if (apply) {
    for (const row of removeRows) {
      try { fs.unlinkSync(row.abs); } catch {}
    }
  }
  return {
    path: targetPath,
    exists: true,
    max_files: keep,
    total_files: total,
    removed_files: removeRows.length,
    applied: apply && removeRows.length > 0
  };
}

function loadPolicy(args: Record<string, any>) {
  const policyPath = resolvePath(args.policy || process.env.RUNTIME_RETENTION_POLICY_PATH || DEFAULT_POLICY_PATH);
  const policy = readJson(policyPath, {});
  const paths = policy && typeof policy.paths === 'object' ? policy.paths : {};
  const statePath = resolvePath(paths.state_path || 'client/runtime/local/state/ops/runtime_retention_prune/latest.json');
  const receiptsPath = resolvePath(paths.receipts_path || 'client/runtime/local/state/ops/runtime_retention_prune/receipts.jsonl');
  const jsonlTargets = Array.isArray(policy.jsonl_targets) ? policy.jsonl_targets : [];
  const dirTargets = Array.isArray(policy.directory_targets) ? policy.directory_targets : [];
  return {
    policyPath,
    enabled: policy && policy.enabled !== false,
    statePath,
    receiptsPath,
    jsonlTargets,
    dirTargets
  };
}

function run() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'run', 20).toLowerCase();
  const apply = toBool(args.apply, false) || cmd === 'apply';
  const runtime = loadPolicy(args);
  if (!runtime.enabled) {
    const out = {
      ok: true,
      type: 'runtime_retention_prune',
      ts: nowIso(),
      command: cmd,
      enabled: false,
      skipped: true,
      reason: 'policy_disabled'
    };
    process.stdout.write(`${JSON.stringify(out)}\n`);
    process.exit(0);
  }
  const jsonlResults = runtime.jsonlTargets.map((target: any) => {
    const keep = toInt(target.keep_lines, 5000, 1, 500000);
    return pruneJsonl(resolvePath(target.path), keep, apply);
  });
  const dirResults = runtime.dirTargets.map((target: any) => {
    const maxFiles = toInt(target.max_files, 500, 1, 100000);
    return pruneDirectory(resolvePath(target.path), maxFiles, apply);
  });
  const removedLines = jsonlResults.reduce((sum: number, row: any) => sum + Number(row.removed_lines || 0), 0);
  const removedFiles = dirResults.reduce((sum: number, row: any) => sum + Number(row.removed_files || 0), 0);
  const out = {
    ok: true,
    type: 'runtime_retention_prune',
    ts: nowIso(),
    command: cmd,
    apply,
    policy_path: runtime.policyPath,
    jsonl_targets: jsonlResults,
    directory_targets: dirResults,
    totals: {
      removed_lines: removedLines,
      removed_files: removedFiles
    }
  };
  out['receipt_hash'] = deterministicHash(out);
  writeJson(runtime.statePath, out);
  appendJsonl(runtime.receiptsPath, out);
  process.stdout.write(`${JSON.stringify(out)}\n`);
  process.exit(0);
}

run();

