#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const SYSTEMS_ROOT = path.join(ROOT, 'client', 'runtime', 'systems');

const DEFAULT_OUT = path.join(ROOT, 'artifacts', 'wrapper_collapse_plan_current.json');
const outArg = process.argv.find((arg) => arg.startsWith('--out='));
const OUT = outArg ? path.resolve(outArg.slice('--out='.length)) : DEFAULT_OUT;

function listTsFiles(dir) {
  const out = [];
  const stack = [dir];
  while (stack.length) {
    const cur = stack.pop();
    for (const ent of fs.readdirSync(cur, { withFileTypes: true })) {
      const abs = path.join(cur, ent.name);
      if (ent.isDirectory()) {
        stack.push(abs);
      } else if (ent.isFile() && abs.endsWith('.ts')) {
        out.push(abs);
      }
    }
  }
  return out.sort();
}

function isWrapperSource(source) {
  return (
    source.includes('createLegacyRetiredModule') ||
    source.includes('createConduitLaneModule') ||
    source.includes('ts_bootstrap')
  );
}

function isPureWrapperSource(source) {
  const lines = source
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith('//'));
  const hasExports = /module\.exports\s*=/.test(source);
  const hasCustomFn =
    /function\s+\w+/.test(source) &&
    !source.includes('createLegacyRetiredModule') &&
    !source.includes('createConduitLaneModule');
  return hasExports && !hasCustomFn && lines.length <= 24;
}

function rel(abs) {
  return path.relative(ROOT, abs).replace(/\\/g, '/');
}

function ensureParent(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

const files = listTsFiles(SYSTEMS_ROOT);
const byDir = new Map();
const wrappers = [];

for (const abs of files) {
  const source = fs.readFileSync(abs, 'utf8');
  const wrapper = isWrapperSource(source);
  const pure = wrapper && isPureWrapperSource(source);
  const item = {
    path: rel(abs),
    wrapper,
    pure,
    loc: source.split(/\r?\n/).length
  };
  const dir = rel(path.dirname(abs));
  const slot = byDir.get(dir) || { dir, total: 0, wrapper: 0, pure: 0, files: [] };
  slot.total += 1;
  if (wrapper) slot.wrapper += 1;
  if (pure) slot.pure += 1;
  slot.files.push(item);
  byDir.set(dir, slot);
  if (wrapper) wrappers.push(item);
}

const dirs = Array.from(byDir.values())
  .map((entry) => ({
    ...entry,
    wrapper_ratio: Number((entry.wrapper / Math.max(1, entry.total)).toFixed(3)),
    pure_ratio: Number((entry.pure / Math.max(1, entry.total)).toFixed(3))
  }))
  .sort((a, b) => {
    if (b.pure !== a.pure) return b.pure - a.pure;
    if (b.wrapper !== a.wrapper) return b.wrapper - a.wrapper;
    return a.dir.localeCompare(b.dir);
  });

const payload = {
  type: 'wrapper_collapse_plan',
  generated_at: new Date().toISOString(),
  scope: rel(SYSTEMS_ROOT),
  summary: {
    total_files: files.length,
    wrapper_files: wrappers.length,
    pure_wrapper_files: wrappers.filter((item) => item.pure).length,
    target_first_tranche: Math.min(250, wrappers.filter((item) => item.pure).length)
  },
  top_directories: dirs.slice(0, 60).map((d) => ({
    dir: d.dir,
    total: d.total,
    wrapper: d.wrapper,
    pure: d.pure,
    wrapper_ratio: d.wrapper_ratio,
    pure_ratio: d.pure_ratio
  })),
  first_tranche_candidates: wrappers
    .filter((item) => item.pure)
    .sort((a, b) => a.loc - b.loc || a.path.localeCompare(b.path))
    .slice(0, 250),
  notes: [
    'Pure wrappers are candidates for collapse behind generic runtime entrypoints.',
    'Do not delete wrappers blindly: migrate call-sites/config references in the same tranche.',
    'Always run policy gates + verify.sh after each collapse batch.'
  ]
};

ensureParent(OUT);
fs.writeFileSync(OUT, JSON.stringify(payload, null, 2) + '\n', 'utf8');
console.log(JSON.stringify(payload.summary, null, 2));
