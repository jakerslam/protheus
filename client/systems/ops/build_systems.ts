#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

function resolveRepoRoot(startDir) {
  let dir = path.resolve(startDir);
  while (true) {
    const pkg = path.join(dir, 'package.json');
    const cargo = path.join(dir, 'Cargo.toml');
    const clientDir = path.join(dir, 'client');
    if (fs.existsSync(pkg) && (fs.existsSync(cargo) || fs.existsSync(clientDir))) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) return path.resolve(startDir, '..', '..');
    dir = parent;
  }
}

function resolveProjectPath(root) {
  const candidates = [
    path.join(root, 'tsconfig.build.json'),
    path.join(root, 'client', 'tsconfig.build.json')
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) return candidate;
  }
  return candidates[0];
}

function resolveLocalTsc(root) {
  const bin = process.platform === 'win32' ? 'tsc.cmd' : 'tsc';
  const candidates = [
    path.join(root, 'node_modules', '.bin', bin),
    path.join(root, 'client', 'node_modules', '.bin', bin)
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) return candidate;
  }
  return candidates[0];
}

const ROOT = resolveRepoRoot(__dirname);
const PROJECT_PATH = resolveProjectPath(ROOT);
const DIST_PATH = path.join(ROOT, 'dist');
const LOCAL_TSC = resolveLocalTsc(ROOT);

function run(bin, args) {
  return spawnSync(bin, args, {
    cwd: ROOT,
    stdio: 'inherit',
    shell: false
  });
}

function main() {
  if (fs.existsSync(DIST_PATH)) {
    fs.rmSync(DIST_PATH, { recursive: true, force: true });
  }

  const args = ['-p', PROJECT_PATH];
  if (fs.existsSync(LOCAL_TSC)) {
    const r = run(LOCAL_TSC, args);
    process.exit(typeof r.status === 'number' ? r.status : 1);
  }

  const r = run('tsc', args);
  if (r.error && r.error.code === 'ENOENT') {
    process.stderr.write('build:systems requires TypeScript. Install with `npm install --save-dev typescript`.\n');
    process.exit(2);
  }
  process.exit(typeof r.status === 'number' ? r.status : 1);
}

if (require.main === module) {
  main();
}
