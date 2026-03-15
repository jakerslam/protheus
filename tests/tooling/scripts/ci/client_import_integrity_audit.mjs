#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';

function listTsFiles(root) {
  const out = [];
  const stack = [root];
  while (stack.length) {
    const cur = stack.pop();
    if (!fs.existsSync(cur)) continue;
    for (const ent of fs.readdirSync(cur, { withFileTypes: true })) {
      const abs = path.join(cur, ent.name);
      if (ent.isDirectory()) {
        if (ent.name === 'node_modules' || ent.name === 'dist' || ent.name === 'target' || ent.name === '.git') continue;
        stack.push(abs);
      } else if (ent.isFile() && abs.endsWith('.ts')) {
        out.push(abs);
      }
    }
  }
  return out.sort();
}

function stripComments(source) {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/(^|[^:])\/\/.*$/gm, '$1');
}

function resolveCandidates(fileDir, spec) {
  const base = spec.startsWith('/') ? spec : path.resolve(fileDir, spec);
  return [
    base,
    `${base}.ts`,
    `${base}.js`,
    `${base}.mjs`,
    `${base}.cjs`,
    path.join(base, 'index.ts'),
    path.join(base, 'index.js')
  ];
}

function normalizeRel(root, abs) {
  return path.relative(root, abs).replace(/\\/g, '/');
}

const cwd = process.cwd();
const strict = process.argv.includes('--strict=1') || process.argv.includes('--strict');
const outArg = process.argv.find((arg) => arg.startsWith('--out='));
const outPath = outArg
  ? path.resolve(outArg.slice('--out='.length))
  : path.join(cwd, 'artifacts', 'client_import_integrity_audit_current.json');

const files = listTsFiles(path.join(cwd, 'client'));
const missing = [];
const re = /require\((['"])([^'"\n]+)\1\)|from\s+(['"])([^'"\n]+)\3|import\((['"])([^'"\n]+)\5\)/g;

for (const abs of files) {
  const file = normalizeRel(cwd, abs);
  const source = stripComments(fs.readFileSync(abs, 'utf8'));
  let match;
  while ((match = re.exec(source)) !== null) {
    const spec = match[2] || match[4] || match[6];
    if (!spec || !(spec.startsWith('./') || spec.startsWith('../') || spec.startsWith('/'))) continue;
    const exists = resolveCandidates(path.dirname(abs), spec).some((candidate) => fs.existsSync(candidate));
    if (!exists) {
      missing.push({ file, spec });
    }
  }
}

const bySpec = Object.create(null);
for (const m of missing) bySpec[m.spec] = (bySpec[m.spec] || 0) + 1;
const topMissingSpecs = Object.entries(bySpec)
  .sort((a, b) => b[1] - a[1])
  .map(([spec, count]) => ({ spec, count }));

const payload = {
  type: 'client_import_integrity_audit',
  generated_at: new Date().toISOString(),
  revision: process.env.GITHUB_SHA || null,
  summary: {
    scanned_files: files.length,
    missing_import_count: missing.length,
    pass: missing.length === 0
  },
  top_missing_specs: topMissingSpecs,
  missing
};

fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
console.log(JSON.stringify(payload, null, 2));

if (strict && missing.length > 0) process.exit(1);
