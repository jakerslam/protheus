#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

const ROOT = process.cwd();
const SCRIPT_PATH = 'tests/tooling/scripts/ci/client_scope_inventory.ts';
const SKIP_DIRS = new Set(['.git', 'node_modules', 'dist', 'coverage', 'state']);

function parseArgs(argv) {
  const out = {
    out: '',
  };
  out.out = cleanText(readFlag(argv, 'out') || '', 400);
  return out;
}

function classify(file) {
  if (file.startsWith('client/runtime/systems/')) return 'runtime_system_surface';
  if (file.startsWith('client/runtime/lib/')) return 'runtime_sdk_surface';
  if (file.startsWith('client/cli/')) return 'cli_surface';
  if (file.startsWith('client/lib/')) return 'sdk_surface';
  if (file.startsWith('client/observability/')) return 'observability_surface';
  if (file.startsWith('client/cognition/')) return 'cognition_surface';
  if (file.startsWith('client/memory/')) return 'memory_surface';
  if (file.startsWith('client/runtime/patches/')) return 'runtime_patch_surface';
  if (file.startsWith('client/runtime/platform/')) return 'platform_surface';
  if (file.startsWith('client/tests/')) return 'misplaced_test_surface';
  if (file.startsWith('client/types/')) return 'type_support_surface';
  return 'other';
}

function walk(dir, out = []) {
  if (!fs.existsSync(dir)) return out;
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, ent.name);
    if (ent.isDirectory()) {
      if (SKIP_DIRS.has(ent.name)) continue;
      walk(p, out);
    } else if (/\.(ts|tsx)$/.test(ent.name)) {
      out.push(p);
    }
  }
  return out;
}

function countBy(entries, fn) {
  const counts = {};
  for (const entry of entries) {
    const key = fn(entry);
    counts[key] = (counts[key] || 0) + 1;
  }
  return Object.fromEntries(Object.entries(counts).sort((a, b) => b[1] - a[1]));
}

function topDirs(files, depth = 3, limit = 40) {
  const counts = {};
  for (const file of files) {
    const parts = file.split('/').slice(0, depth);
    const key = parts.join('/');
    counts[key] = (counts[key] || 0) + 1;
  }
  return Object.entries(counts)
    .sort((a, b) => b[1] - a[1])
    .slice(0, limit)
    .map(([dir, count]) => ({ dir, count }));
}

function buildReport(root = ROOT) {
  const revision = currentRevision(root);
  const files = walk(path.resolve(root, 'client'))
    .map((file) => path.relative(root, file).replace(/\\/g, '/'))
    .filter((file) => !file.startsWith('client/runtime/local/'));

  const entries = files.map((file) => ({
    file,
    category: classify(file),
  }));

  return {
    type: 'client_scope_inventory',
    generated_at: new Date().toISOString(),
    revision,
    summary: {
      total_ts_files: entries.length,
      by_category: countBy(entries, (entry) => entry.category),
      top_dirs: topDirs(files),
    },
    entries,
  };
}

function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const payload = buildReport(ROOT);
  return emitStructuredResult(payload, {
    outPath: args.out || undefined,
    strict: false,
    ok: true,
    history: false,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  SCRIPT_PATH,
  SKIP_DIRS,
  parseArgs,
  classify,
  walk,
  countBy,
  topDirs,
  buildReport,
  run,
};
