#!/usr/bin/env node
// Layer ownership: client/cognition/habits/scripts (authoritative)
// Uses the public protheus-ops CLI contract rather than private runtime paths.

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');

function isFile(filePath) {
  try {
    return fs.statSync(filePath).isFile();
  } catch {
    return false;
  }
}

function resolveBinary() {
  const explicit = String(process.env.PROTHEUS_NPM_BINARY || '').trim();
  if (explicit && isFile(explicit)) return explicit;

  const exe = process.platform === 'win32' ? 'protheus-ops.exe' : 'protheus-ops';
  const vendor = path.join(ROOT, 'client', 'cli', 'npm', 'vendor', exe);
  if (isFile(vendor)) return vendor;

  const release = path.join(ROOT, 'target', 'release', exe);
  if (isFile(release)) return release;

  const debug = path.join(ROOT, 'target', 'debug', exe);
  if (isFile(debug)) return debug;

  throw new Error('protheus-ops binary not found; set PROTHEUS_NPM_BINARY or build target/{release,debug}/protheus-ops');
}

function runProtheusOps(args) {
  const bin = resolveBinary();
  const proc = spawnSync(bin, args, {
    cwd: ROOT,
    stdio: 'inherit',
    env: { ...process.env, PROTHEUS_ROOT: ROOT },
  });
  return Number.isFinite(proc.status) ? proc.status : 1;
}

const date = process.argv[2];
const maxEyesArg = process.argv.find(a => a.startsWith("--max-eyes="));

// Wrappers run with infra clearance (tier 3) by default
if (!process.env.CLEARANCE) process.env.CLEARANCE = "3";

const args = ["spine", "daily"];
if (date) args.push(date);
if (maxEyesArg) args.push(maxEyesArg);

process.exit(runProtheusOps(args));
