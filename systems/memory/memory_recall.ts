#!/usr/bin/env node
'use strict';
export {};

/**
 * V6 memory recall adapter.
 * Delegates recall/get/cache-clear operations to crates/memory Rust core.
 */

const path = require('path');
const { spawnSync } = require('child_process');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  emit
} = require('../../lib/queued_backlog_runtime');

const POLICY_PATH = process.env.MEMORY_RECALL_POLICY_PATH
  ? path.resolve(process.env.MEMORY_RECALL_POLICY_PATH)
  : path.join(ROOT, 'config', 'memory_recall_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/memory/memory_recall.js query --q="..." [--top=5]');
  console.log('  node systems/memory/memory_recall.js get --id=<memory_id>');
  console.log('  node systems/memory/memory_recall.js clear-cache');
  console.log('  node systems/memory/memory_recall.js status');
}

function policy() {
  const base = {
    enabled: true,
    rust_manifest: 'crates/memory/Cargo.toml',
    rust_bin: 'memory-cli',
    paths: {
      latest_path: 'state/memory/runtime_audit/memory_recall_latest.json',
      receipts_path: 'state/memory/runtime_audit/memory_recall_receipts.jsonl'
    }
  };
  const raw = readJson(POLICY_PATH, {});
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    enabled: raw.enabled !== false,
    rust_manifest: resolvePath(raw.rust_manifest || base.rust_manifest, base.rust_manifest),
    rust_bin: cleanText(raw.rust_bin || base.rust_bin, 120) || base.rust_bin,
    paths: {
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path)
    }
  };
}

function parseJson(text: string) {
  const raw = String(text || '').trim();
  if (!raw) return null;
  try { return JSON.parse(raw); } catch {}
  const lines = raw.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try { return JSON.parse(lines[i]); } catch {}
  }
  return null;
}

function runRust(args: string[], timeoutMs = 180000) {
  const started = Date.now();
  const cmd = [
    'cargo',
    'run',
    '--quiet',
    '--manifest-path',
    'crates/memory/Cargo.toml',
    '--bin',
    'memory-cli',
    '--',
    ...args
  ];
  const out = spawnSync(cmd[0], cmd.slice(1), {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: Math.max(1000, timeoutMs)
  });
  const status = Number.isFinite(Number(out.status)) ? Number(out.status) : 1;
  return {
    ok: status === 0,
    status,
    duration_ms: Math.max(0, Date.now() - started),
    stderr: cleanText(out.stderr || '', 500),
    payload: parseJson(String(out.stdout || '')),
    command: cmd
  };
}

function writeReceipt(p: any, receipt: any) {
  writeJsonAtomic(p.paths.latest_path, receipt);
  appendJsonl(p.paths.receipts_path, receipt);
}

function cmdQuery(args: any, p: any) {
  const q = cleanText(args.q || args.query || '', 400);
  const top = Number.isFinite(Number(args.top)) ? Math.max(1, Number(args.top)) : 5;
  const run = runRust([`recall`, `--query=${q}`, `--limit=${top}`]);
  const payload = run.payload || {};
  const receipt = {
    ts: nowIso(),
    type: 'memory_recall_query',
    ok: run.ok && payload && payload.ok === true,
    backend: 'rust_core_v6',
    command_status: run.status,
    duration_ms: run.duration_ms,
    query: q,
    top,
    hit_count: Number(payload.hit_count || 0),
    hits: Array.isArray(payload.hits) ? payload.hits : [],
    error: payload.error || (run.ok ? null : (run.stderr || 'rust_command_failed'))
  };
  writeReceipt(p, receipt);
  return receipt;
}

function cmdGet(args: any, p: any) {
  const id = cleanText(args.id || args['node-id'] || args.uid || '', 200);
  const run = runRust([`get`, `--id=${id}`]);
  const payload = run.payload || {};
  const receipt = {
    ts: nowIso(),
    type: 'memory_recall_get',
    ok: run.ok && payload && payload.ok === true,
    backend: 'rust_core_v6',
    command_status: run.status,
    duration_ms: run.duration_ms,
    id,
    row: payload.row || null,
    error: payload.error || (run.ok ? null : (run.stderr || 'rust_command_failed'))
  };
  writeReceipt(p, receipt);
  return receipt;
}

function cmdClearCache(p: any) {
  const run = runRust(['clear-cache']);
  const payload = run.payload || {};
  const receipt = {
    ts: nowIso(),
    type: 'memory_recall_clear_cache',
    ok: run.ok && payload && payload.ok === true,
    backend: 'rust_core_v6',
    command_status: run.status,
    duration_ms: run.duration_ms,
    cleared: Number(payload.cleared || 0),
    error: payload.error || (run.ok ? null : (run.stderr || 'rust_command_failed'))
  };
  writeReceipt(p, receipt);
  return receipt;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 80) || 'status';
  if (cmd === '--help' || cmd === 'help' || cmd === '-h') {
    usage();
    process.exit(0);
  }
  const p = policy();
  if (!p.enabled) emit({ ok: false, error: 'memory_recall_disabled' }, 1);

  if (cmd === 'query') emit(cmdQuery(args, p), 0);
  if (cmd === 'get') emit(cmdGet(args, p), 0);
  if (cmd === 'clear-cache') emit(cmdClearCache(p), 0);
  if (cmd === 'status') emit({ ok: true, type: 'memory_recall_status', latest: readJson(p.paths.latest_path, null) }, 0);

  emit({ ok: false, error: 'unsupported_command', cmd }, 1);
}

main();
