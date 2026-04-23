'use strict';

// legacy_process_runner_dev_only

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

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
  let newest = Math.max(mtimeMs(path.join(root, 'Cargo.toml')), mtimeMs(path.join(opsRoot, 'Cargo.toml')));
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

function allowStaleBinary(env = process.env) {
  const raw = String(
    (env && (env.INFRING_OPS_ALLOW_STALE || env.INFRING_NPM_ALLOW_STALE)) || ''
  )
    .trim()
    .toLowerCase();
  return raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on';
}

function resolveBinary(root, options = {}) {
  const env = options && options.env ? options.env : process.env;
  const allowStale = allowStaleBinary(env);
  const explicit = String((env && env.INFRING_NPM_BINARY) || process.env.INFRING_NPM_BINARY || '').trim();
  const explicitExists = explicit && isFile(explicit);
  const explicitResolved = explicitExists ? path.resolve(explicit) : '';
  const release = path.join(
    root,
    'target',
    'release',
    process.platform === 'win32' ? 'infring-ops.exe' : 'infring-ops'
  );
  const target = path.join(
    root,
    'target',
    'debug',
    process.platform === 'win32' ? 'infring-ops.exe' : 'infring-ops'
  );
  const vendor = path.join(
    root,
    'client',
    'cli',
    'npm',
    'vendor',
    process.platform === 'win32' ? 'infring-ops.exe' : 'infring-ops'
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

function spawnInvocation(root, command, args, env) {
  return spawnSync(command, args, {
    cwd: root,
    encoding: 'utf8',
    stdio: 'pipe',
    env,
    maxBuffer: 128 * 1024 * 1024,
  });
}

function processStatus(proc) {
  if (!proc) return 1;
  if (proc.error) return 1;
  return Number.isFinite(proc.status) ? proc.status : 1;
}

function processOutput(proc) {
  const stdout = proc && typeof proc.stdout === 'string' ? proc.stdout : '';
  const stderrBase = proc && typeof proc.stderr === 'string' ? proc.stderr : '';
  const err = proc && proc.error ? `\n${String(proc.error.message || proc.error)}` : '';
  return {
    stdout,
    stderr: `${stderrBase}${err}`,
    combined: `${stdout}\n${stderrBase}${err}`,
  };
}

function writeAll(fd, text) {
  if (!text) return;
  const buffer = Buffer.from(text, 'utf8');
  let offset = 0;
  while (offset < buffer.length) {
    offset += fs.writeSync(fd, buffer, offset, buffer.length - offset);
  }
}

function emitProcessOutput(proc) {
  const out = processOutput(proc);
  writeAll(1, out.stdout);
  writeAll(2, out.stderr);
}

function shouldFallbackToCargo(args, proc, options = {}) {
  if (options.unknownDomainFallback === false) return false;
  if (!Array.isArray(args) || args.length === 0) return false;
  if (processStatus(proc) === 0) return false;
  const out = processOutput(proc);
  return /\bunknown_domain\b/i.test(out.combined);
}

function runViaCargo(root, args, env) {
  return spawnInvocation(
    root,
    'cargo',
    ['run', '--quiet', '-p', 'infring-ops-core', '--bin', 'infring-ops', '--'].concat(args),
    env
  );
}

function runLegacyProcessRunner(root, args, options = {}) {
  const env = { ...process.env, INFRING_ROOT: root, ...(options.env || {}) };
  const bin = resolveBinary(root, { env });
  if (bin) {
    const proc = spawnInvocation(root, bin, args, env);
    if (shouldFallbackToCargo(args, proc, options)) {
      const fallback = runViaCargo(root, args, env);
      if (!fallback.error) {
        emitProcessOutput(fallback);
        return processStatus(fallback);
      }
    }
    emitProcessOutput(proc);
    return processStatus(proc);
  }
  const proc = runViaCargo(root, args, env);
  emitProcessOutput(proc);
  return processStatus(proc);
}

module.exports = {
  resolveBinary,
  runLegacyProcessRunner,
};
