'use strict';

// Layer ownership: adapters/runtime (binary resolution helper).
//
// Canonical home for resolving the `infring-ops` Rust binary. Migrated out of
// adapters/runtime/dev_only/legacy_process_runner.ts as part of V11-OPS-PRD-001
// (legacy runner deletion milestone v0.3.11-stable / 2026-05-15) so binary
// resolution survives the legacy runner deletion.
//
// This module:
//   - is non-dev-only (must remain after dev_only/ is deleted in PR2),
//   - has no spawn/exec calls (resolution only — invocation lives in the bridge),
//   - exposes a single `resolveBinary(root, options?)` entry point.

const fs = require('fs');
const path = require('path');

function isFile(filePath) {
  try {
    return fs.statSync(filePath).isFile();
  } catch {
    return false;
  }
}

function mtimeMs(filePath) {
  try {
    return fs.statSync(filePath).mtimeMs || 0;
  } catch {
    return 0;
  }
}

function sourceNewestMtimeMs(root) {
  const opsRoot = path.join(root, 'core', 'layer0', 'ops');
  const srcRoot = path.join(opsRoot, 'src');
  let newest = Math.max(
    mtimeMs(path.join(root, 'Cargo.toml')),
    mtimeMs(path.join(opsRoot, 'Cargo.toml')),
  );
  const stack = [srcRoot];
  while (stack.length > 0) {
    const dir = stack.pop();
    let entries = [];
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        stack.push(fullPath);
        continue;
      }
      if (entry.isFile() && fullPath.endsWith('.rs')) {
        newest = Math.max(newest, mtimeMs(fullPath));
      }
    }
  }
  return newest;
}

function binaryFreshEnough(root, binPath) {
  const binMtime = mtimeMs(binPath);
  if (!binMtime) return false;
  const srcMtime = sourceNewestMtimeMs(root);
  if (!srcMtime) return true;
  return binMtime >= srcMtime;
}

function allowStaleBinary(env) {
  const source = env || process.env;
  const raw = String(
    (source && (source.INFRING_OPS_ALLOW_STALE || source.INFRING_NPM_ALLOW_STALE)) || '',
  )
    .trim()
    .toLowerCase();
  return raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on';
}

function resolveBinary(root, options = {}) {
  const env = options && options.env ? options.env : process.env;
  const allowStale = allowStaleBinary(env);
  const explicit = String((env && env.INFRING_NPM_BINARY) || process.env.INFRING_NPM_BINARY || '')
    .trim();
  const explicitExists = explicit && isFile(explicit);
  const explicitResolved = explicitExists ? path.resolve(explicit) : '';
  const release = path.join(
    root,
    'target',
    'release',
    process.platform === 'win32' ? 'infring-ops.exe' : 'infring-ops',
  );
  const target = path.join(
    root,
    'target',
    'debug',
    process.platform === 'win32' ? 'infring-ops.exe' : 'infring-ops',
  );
  const vendor = path.join(
    root,
    'client',
    'cli',
    'npm',
    'vendor',
    process.platform === 'win32' ? 'infring-ops.exe' : 'infring-ops',
  );
  const candidates = [release, target, vendor]
    .filter((binPath) => isFile(binPath))
    .filter((binPath) => allowStale || binaryFreshEnough(root, binPath))
    .map((binPath) => ({ binPath, mtime: mtimeMs(binPath) }))
    .sort((a, b) => b.mtime - a.mtime);
  const localBest = candidates.length > 0 ? candidates[0].binPath : null;
  if (explicitExists && explicitResolved.startsWith(root)) return explicitResolved;
  if (localBest) return localBest;
  if (explicitExists) return explicitResolved;
  return null;
}

module.exports = {
  resolveBinary,
};
