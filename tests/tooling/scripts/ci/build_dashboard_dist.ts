#!/usr/bin/env node
/* eslint-disable no-console */
'use strict';

const fs = require('fs');
const path = require('path');
const childProcess = require('child_process');
const esbuild = require('esbuild');

function repoRoot(startDir = __dirname) {
  let dir = path.resolve(startDir);
  while (true) {
    const cargo = path.join(dir, 'Cargo.toml');
    const coreOps = path.join(dir, 'core', 'layer0', 'ops', 'Cargo.toml');
    if (fs.existsSync(cargo) && fs.existsSync(coreOps)) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return path.resolve(__dirname, '..', '..', '..', '..');
}

function parseArgs(argv) {
  const out = {
    minify: false
  };
  for (const raw of argv) {
    const token = String(raw || '').trim();
    if (!token) continue;
    if (token === '--minify' || token === '--minify=1') {
      out.minify = true;
    }
  }
  return out;
}

function copyDirRecursive(src, dest) {
  if (!fs.existsSync(src)) return;
  fs.mkdirSync(dest, { recursive: true });
  for (const entry of fs.readdirSync(src, { withFileTypes: true })) {
    const from = path.join(src, entry.name);
    const to = path.join(dest, entry.name);
    if (entry.isDirectory()) {
      copyDirRecursive(from, to);
    } else if (entry.isFile()) {
      fs.copyFileSync(from, to);
    }
  }
}

function runCommand(bin, args, cwd) {
  const cmd = process.platform === 'win32' && bin === 'npm' ? 'npm.cmd' : bin;
  const result = childProcess.spawnSync(cmd, args, {
    cwd,
    encoding: 'utf8',
    stdio: 'pipe'
  });
  if (result && result.status === 0) return;
  const stderr = String((result && result.stderr) || '').trim();
  const stdout = String((result && result.stdout) || '').trim();
  const detail = stderr || stdout || 'unknown_error';
  throw new Error(`${bin}_failed:${detail}`);
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const root = repoRoot(__dirname);
  const entry = path.join(root, 'client', 'runtime', 'systems', 'ui', 'infring_dashboard.ts');
  const outfile = path.join(root, 'dist', 'client', 'runtime', 'systems', 'ui', 'infring_dashboard.js');
  const staticSrc = path.join(root, 'client', 'runtime', 'systems', 'ui', 'infring_static');
  const staticDest = path.join(root, 'dist', 'client', 'runtime', 'systems', 'ui', 'infring_static');
  fs.mkdirSync(path.dirname(outfile), { recursive: true });
  await esbuild.build({
    entryPoints: [entry],
    outfile,
    bundle: true,
    platform: 'node',
    format: 'cjs',
    target: 'node22',
    sourcemap: false,
    minify: options.minify,
    logLevel: 'silent',
    legalComments: 'none',
    define: {
      'process.env.NODE_ENV': JSON.stringify('production')
    }
  });
  fs.rmSync(staticDest, { recursive: true, force: true });
  copyDirRecursive(staticSrc, staticDest);
  const bytes = fs.statSync(outfile).size;
  const payload = {
    ok: true,
    type: 'dashboard_dist_build',
    entry: path.relative(root, entry).replace(/\\/g, '/'),
    out_file: path.relative(root, outfile).replace(/\\/g, '/'),
    static_dir: path.relative(root, staticDest).replace(/\\/g, '/'),
    out_bytes: bytes,
    minify: options.minify
  };
  process.stdout.write(`${JSON.stringify(payload)}\n`);
}

main().catch((error) => {
  const payload = {
    ok: false,
    type: 'dashboard_dist_build_failed',
    error: String((error && error.message) || error || 'unknown_error')
  };
  process.stderr.write(`${JSON.stringify(payload)}\n`);
  process.exit(1);
});
