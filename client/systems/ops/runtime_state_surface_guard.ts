#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const {
  ROOT,
  nowIso,
  parseArgs,
  toBool,
  cleanText,
  writeJsonAtomic,
  appendJsonl,
  emit,
  resolvePath
} = require('../../lib/queued_backlog_runtime');
const { CANONICAL_PATHS, LEGACY_SURFACES } = require('../../lib/runtime_path_registry');

function usage() {
  console.log('Usage:');
  console.log('  node client/systems/ops/runtime_state_surface_guard.js check [--strict=1|0]');
  console.log('  node client/systems/ops/runtime_state_surface_guard.js status');
}

function defaultPolicy() {
  return {
    version: '2.0',
    strict_default: true,
    disallowed_runtime_surfaces: LEGACY_SURFACES.slice(),
    required_runtime_roots: [
      CANONICAL_PATHS.client_state_root,
      CANONICAL_PATHS.core_state_root
    ],
    outputs: {
      latest_path: 'client/local/state/ops/runtime_state_surface_guard/latest.json',
      receipts_path: 'client/local/state/ops/runtime_state_surface_guard/receipts.jsonl'
    }
  };
}

function rel(absPath: string) {
  return path.relative(ROOT, absPath).replace(/\\/g, '/');
}

function trackedPaths(prefix: string) {
  const run = spawnSync('git', ['ls-files', '--', prefix], {
    cwd: ROOT,
    encoding: 'utf8'
  });
  if (run.status !== 0) return [];
  return String(run.stdout || '')
    .split('\n')
    .map((row) => cleanText(row, 520).replace(/\\/g, '/'))
    .filter((row) => row && fs.existsSync(path.join(ROOT, row)))
    .filter(Boolean);
}

function pathStatus(relPath: string) {
  const abs = path.join(ROOT, relPath);
  if (!fs.existsSync(abs)) {
    return { path: relPath, exists: false, symlink: false, kind: null };
  }
  let stat: any = null;
  try {
    stat = fs.lstatSync(abs);
  } catch {
    return { path: relPath, exists: true, symlink: false, kind: 'stat_failed' };
  }
  const isSym = stat.isSymbolicLink();
  if (isSym) {
    let target = '';
    try {
      target = cleanText(fs.readlinkSync(abs), 320);
    } catch {}
    return { path: relPath, exists: true, symlink: true, kind: 'symlink', target };
  }
  const kind = stat.isDirectory() ? 'directory' : stat.isFile() ? 'file' : 'other';
  return { path: relPath, exists: true, symlink: false, kind };
}

function runCheck(strict: boolean) {
  const policy = defaultPolicy();
  const disallowed: string[] = Array.isArray(policy.disallowed_runtime_surfaces)
    ? policy.disallowed_runtime_surfaces.map((x) => cleanText(x, 160)).filter(Boolean)
    : [];
  const required: string[] = Array.isArray(policy.required_runtime_roots)
    ? policy.required_runtime_roots.map((x) => cleanText(x, 160)).filter(Boolean)
    : [];

  const surfaceResults = disallowed.map((surface) => pathStatus(surface));
  const requiredResults = required.map((surface) => pathStatus(surface));

  const trackedBySurface = disallowed.map((surface) => ({
    surface,
    entries: trackedPaths(surface)
  }));
  const illegalTracked = trackedBySurface
    .flatMap((row) => row.entries.map((entry: string) => ({ surface: row.surface, entry })));

  const checks = {
    no_legacy_runtime_surfaces_present: surfaceResults.every((row) => row.exists === false),
    no_tracked_legacy_runtime_surfaces: illegalTracked.length === 0,
    canonical_runtime_roots_present: requiredResults.every((row) => row.exists === true)
  };

  const blocking = Object.entries(checks)
    .filter(([, ok]) => ok !== true)
    .map(([name]) => name);

  const pass = blocking.length === 0;
  const out = {
    ok: strict ? pass : true,
    pass,
    strict,
    type: 'runtime_state_surface_guard',
    ts: nowIso(),
    checks,
    blocking_checks: blocking,
    disallowed_surface_results: surfaceResults,
    required_root_results: requiredResults,
    tracked_disallowed_surfaces: trackedBySurface,
    illegal_tracked_entries: illegalTracked
  };
  const latestPath = resolvePath(policy.outputs.latest_path, policy.outputs.latest_path);
  const receiptsPath = resolvePath(policy.outputs.receipts_path, policy.outputs.receipts_path);
  writeJsonAtomic(latestPath, out);
  appendJsonl(receiptsPath, out);
  return out;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'check', 40).toLowerCase();
  if (args.help || cmd === '--help' || cmd === 'help' || cmd === '-h') {
    usage();
    return emit({ ok: true, help: true }, 0);
  }
  if (cmd === 'status') {
    const policy = defaultPolicy();
    const latestPath = resolvePath(policy.outputs.latest_path, policy.outputs.latest_path);
    let latest = null;
    try {
      latest = fs.existsSync(latestPath) ? JSON.parse(fs.readFileSync(latestPath, 'utf8')) : null;
    } catch {
      latest = null;
    }
    return emit({
      ok: true,
      type: 'runtime_state_surface_guard_status',
      ts: nowIso(),
      latest
    }, 0);
  }
  if (cmd !== 'check') {
    usage();
    return emit({ ok: false, error: `unknown_command:${cmd}` }, 1);
  }
  const strict = toBool(args.strict, defaultPolicy().strict_default);
  const out = runCheck(strict);
  return emit(out, out.ok ? 0 : 1);
}

main();
