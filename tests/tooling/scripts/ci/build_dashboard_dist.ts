#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import esbuild from 'esbuild';
import { cleanText, hasFlag, parseBool, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult } from '../../lib/result.ts';
const { buildDashboardSvelteIslands } = require('./build_dashboard_svelte_islands.ts');

const SCRIPT_PATH = 'tests/tooling/scripts/ci/build_dashboard_dist.ts';

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
    minify: false,
    out: '',
  };
  out.minify = hasFlag(argv, 'minify') || parseBool(readFlag(argv, 'minify'), false);
  out.out = cleanText(readFlag(argv, 'out') || '', 400);
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

async function buildDashboardDist(options = {}, root = repoRoot(__dirname)) {
  const minify = Boolean(options && options.minify);
  await buildDashboardSvelteIslands({ minify: true }, root);
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
    minify,
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
    minify,
  };
  return payload;
}

async function run(argv = process.argv.slice(2)) {
  const options = parseArgs(argv);
  try {
    const payload = await buildDashboardDist({ minify: options.minify });
    emitStructuredResult(payload, {
      outPath: options.out || undefined,
      strict: false,
      ok: true,
      history: false,
      stdout: false,
    });
    process.stdout.write(`${JSON.stringify(payload)}\n`);
    return 0;
  } catch (error) {
    const payload = {
      ok: false,
      type: 'dashboard_dist_build_failed',
      error: String((error && error.message) || error || 'unknown_error'),
    };
    emitStructuredResult(payload, {
      outPath: options.out || undefined,
      strict: true,
      ok: false,
      history: false,
      stdout: false,
    });
    process.stderr.write(`${JSON.stringify(payload)}\n`);
    return 1;
  }
}

if (require.main === module) {
  run(process.argv.slice(2)).then((code) => process.exit(code));
}

module.exports = {
  SCRIPT_PATH,
  repoRoot,
  parseArgs,
  copyDirRecursive,
  buildDashboardDist,
  run,
};
