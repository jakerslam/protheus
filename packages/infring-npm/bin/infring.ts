#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const { sanitizeBridgeArg } = require('../../../client/runtime/lib/runtime_system_entrypoint.ts');
const MAX_ARG_LEN = 512;

function sanitizeArgToken(value, maxLen = MAX_ARG_LEN) {
  const max = Math.max(1, Number(maxLen) || 1);
  return sanitizeBridgeArg(value, max);
}

function isFile(filePath) {
  try { return fs.statSync(filePath).isFile(); } catch { return false; }
}

function resolveExecutableName() {
  return process.platform === 'win32' ? 'infring-ops.exe' : 'infring-ops';
}

function findBinary() {
  const exe = resolveExecutableName();
  const aliasExe = process.platform === 'win32' ? 'infring-ops.exe' : 'infring-ops';
  const pkgRoot = path.resolve(__dirname, '..');
  for (const candidate of [path.join(pkgRoot, 'vendor', exe), path.join(pkgRoot, 'vendor', aliasExe)]) {
    if (isFile(candidate)) return candidate;
  }
  const envPath = sanitizeArgToken(process.env.INFRING_NPM_BINARY || '', 1024);
  if (envPath && isFile(envPath)) return envPath;
  const repoRoot = path.resolve(pkgRoot, '..', '..');
  for (const candidate of [
    path.join(repoRoot, 'target', 'debug', exe),
    path.join(repoRoot, 'target', 'release', exe),
    path.join(repoRoot, 'target', 'debug', aliasExe),
    path.join(repoRoot, 'target', 'release', aliasExe)
  ]) {
    if (isFile(candidate)) return candidate;
  }
  return null;
}

function hasRuntimeAssets(rootDir) {
  if (!rootDir) return false;
  return [
    path.join(rootDir, 'client', 'runtime', 'systems', 'ops', 'infringctl.js'),
    path.join(rootDir, 'client', 'runtime', 'systems', 'ops', 'infringctl.ts'),
    path.join(rootDir, 'runtime', 'systems', 'ops', 'infringctl.js'),
    path.join(rootDir, 'core', 'layer0', 'ops', 'Cargo.toml'),
  ].some((p) => isFile(p));
}

function resolveRuntimeRoot(pkgRoot) {
  const explicit = sanitizeArgToken(process.env.INFRING_ROOT || '', 1024);
  if (explicit && hasRuntimeAssets(explicit)) return explicit;
  const cwd = process.cwd();
  if (hasRuntimeAssets(cwd)) return cwd;
  const repoRootCandidate = path.resolve(pkgRoot, '..', '..');
  if (hasRuntimeAssets(repoRootCandidate)) return repoRootCandidate;
  if (hasRuntimeAssets(pkgRoot)) return pkgRoot;
  return null;
}

function run() {
  const pkgRoot = path.resolve(__dirname, '..');
  const binPath = findBinary();
  if (!binPath) {
    process.stderr.write('infring npm binary is missing. Reinstall package or run npm rebuild infring.\n');
    process.exit(1);
  }

  const runtimeRoot = resolveRuntimeRoot(pkgRoot);
  const args = (process.argv.slice(2) || []).map((arg) => sanitizeArgToken(arg)).filter(Boolean);
  const env = { ...process.env };

  let finalArgs;
  if (runtimeRoot) {
    env.INFRING_ROOT = runtimeRoot;
    finalArgs = ['infringctl', ...args];
  } else {
    env.INFRING_CTL_SECURITY_GATE_DISABLED = '1';
    finalArgs = args.length ? args : ['--help'];
  }

  const out = spawnSync(binPath, finalArgs, {
    stdio: 'inherit',
    env,
    cwd: runtimeRoot || process.cwd(),
  });

  if (out && out.error) {
    process.stderr.write(JSON.stringify({ ok: false, type: 'infring_npm_bin', error: 'spawn_failed', detail: sanitizeArgToken(out.error.message || out.error, 240) }) + '\n');
  }

  process.exit(Number.isFinite(out.status) ? out.status : 1);
}

run();
