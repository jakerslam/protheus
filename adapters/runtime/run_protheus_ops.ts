#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/runtime (shared app bridge helper)

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const { createOpsLaneBridge } = require('./ops_lane_bridge.ts');

const ROOT = path.resolve(__dirname, '..', '..');

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

function sourceNewestMtimeMs() {
  const opsRoot = path.join(ROOT, 'core', 'layer0', 'ops');
  const srcRoot = path.join(opsRoot, 'src');
  let newest = Math.max(
    mtimeMs(path.join(ROOT, 'Cargo.toml')),
    mtimeMs(path.join(opsRoot, 'Cargo.toml'))
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

function binaryFreshEnough(binPath) {
  const binMtime = mtimeMs(binPath);
  if (!binMtime) return false;
  const srcMtime = sourceNewestMtimeMs();
  if (!srcMtime) return true;
  return binMtime >= srcMtime;
}

function allowStaleBinary(env = process.env) {
  const raw = String(
    (env && (env.PROTHEUS_OPS_ALLOW_STALE || env.PROTHEUS_NPM_ALLOW_STALE)) || ''
  )
    .trim()
    .toLowerCase();
  return raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on';
}

function envTrue(value) {
  const raw = String(value || '').trim().toLowerCase();
  return raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on';
}

function releaseChannel(env = process.env) {
  const raw = String((env && (env.INFRING_RELEASE_CHANNEL || env.PROTHEUS_RELEASE_CHANNEL)) || '')
    .trim()
    .toLowerCase();
  return raw || 'stable';
}

function isProductionReleaseChannel(channel) {
  const normalized = String(channel || '').trim().toLowerCase();
  return (
    normalized === 'stable' ||
    normalized === 'production' ||
    normalized === 'prod' ||
    normalized === 'ga' ||
    normalized === 'release'
  );
}

function withScopedEnv(overrides, fn) {
  const keys = Object.keys(overrides || {});
  if (keys.length === 0) {
    return fn();
  }
  const previous = {};
  for (const key of keys) {
    previous[key] = Object.prototype.hasOwnProperty.call(process.env, key)
      ? process.env[key]
      : undefined;
    const value = overrides[key];
    if (value === undefined || value === null || value === '') {
      delete process.env[key];
    } else {
      process.env[key] = String(value);
    }
  }
  try {
    return fn();
  } finally {
    for (const key of keys) {
      const value = previous[key];
      if (value === undefined) delete process.env[key];
      else process.env[key] = value;
    }
  }
}

function legacyProcessRunnerForced(env = process.env) {
  const forced = envTrue(
    (env && (env.INFRING_OPS_FORCE_LEGACY_PROCESS_RUNNER || env.PROTHEUS_OPS_FORCE_LEGACY_PROCESS_RUNNER)) || ''
  );
  if (!forced) return false;
  return !isProductionReleaseChannel(releaseChannel(env));
}

function resolveBinary(options = {}) {
  const env = options && options.env ? options.env : process.env;
  const allowStale = allowStaleBinary(env);
  const explicit = String((env && env.PROTHEUS_NPM_BINARY) || process.env.PROTHEUS_NPM_BINARY || '').trim();
  const explicitExists = explicit && isFile(explicit);
  const explicitResolved = explicitExists ? path.resolve(explicit) : '';

  const release = path.join(
    ROOT,
    'target',
    'release',
    process.platform === 'win32' ? 'protheus-ops.exe' : 'protheus-ops'
  );
  const target = path.join(
    ROOT,
    'target',
    'debug',
    process.platform === 'win32' ? 'protheus-ops.exe' : 'protheus-ops'
  );
  const vendor = path.join(
    ROOT,
    'client',
    'cli',
    'npm',
    'vendor',
    process.platform === 'win32' ? 'protheus-ops.exe' : 'protheus-ops'
  );

  const candidates = [release, target, vendor]
    .filter((binPath) => isFile(binPath))
    .filter((binPath) => allowStale || binaryFreshEnough(binPath))
    .map((binPath) => ({ binPath, mtime: mtimeMs(binPath) }))
    .sort((a, b) => b.mtime - a.mtime);
  const localBest = candidates.length > 0 ? candidates[0].binPath : null;

  // Prefer workspace-local binaries when available. This prevents stale global
  // install pins (for example ~/.local/bin/protheus-ops) from shadowing fresh
  // core features required by the active workspace UI runtime.
  if (explicitExists && explicitResolved.startsWith(ROOT)) return explicitResolved;
  if (localBest) return localBest;
  if (explicitExists) return explicitResolved;

  return null;
}

function spawnInvocation(
  command,
  args,
  env
) {
  return spawnSync(command, args, {
    cwd: ROOT,
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
    combined: `${stdout}\n${stderrBase}${err}`
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

function shouldFallbackToCargo(
  args,
  proc,
  options = {}
) {
  if (options.unknownDomainFallback === false) return false;
  if (!Array.isArray(args) || args.length === 0) return false;
  if (processStatus(proc) === 0) return false;
  const out = processOutput(proc);
  return /\bunknown_domain\b/i.test(out.combined);
}

function runViaCargo(args, env) {
  return spawnInvocation(
    'cargo',
    ['run', '--quiet', '-p', 'protheus-ops-core', '--bin', 'protheus-ops', '--'].concat(args),
    env
  );
}

function runProtheusOpsViaBridge(args, options = {}) {
  if (!Array.isArray(args) || args.length === 0) return null;
  const domain = String(args[0] || '').trim();
  if (!domain || domain.startsWith('-')) return null;

  const passArgs = args.slice(1);
  const envOverrides = {};
  if (options.unknownDomainFallback === false) {
    envOverrides.INFRING_OPS_ALLOW_CARGO_FALLBACK = '0';
    envOverrides.PROTHEUS_OPS_ALLOW_CARGO_FALLBACK = '0';
  }
  const productionRelease = isProductionReleaseChannel(releaseChannel(process.env));
  if (options.allowProcessFallback === true && !productionRelease) {
    envOverrides.INFRING_OPS_ALLOW_PROCESS_FALLBACK = '1';
    envOverrides.PROTHEUS_OPS_ALLOW_PROCESS_FALLBACK = '1';
  } else if (options.allowProcessFallback === false) {
    envOverrides.INFRING_OPS_ALLOW_PROCESS_FALLBACK = '0';
    envOverrides.PROTHEUS_OPS_ALLOW_PROCESS_FALLBACK = '0';
  } else if (
    !Object.prototype.hasOwnProperty.call(process.env, 'INFRING_OPS_ALLOW_PROCESS_FALLBACK') &&
    !Object.prototype.hasOwnProperty.call(process.env, 'PROTHEUS_OPS_ALLOW_PROCESS_FALLBACK')
  ) {
    // Bridge-first default: keep process fallback disabled unless explicitly requested.
    envOverrides.INFRING_OPS_ALLOW_PROCESS_FALLBACK = '0';
    envOverrides.PROTHEUS_OPS_ALLOW_PROCESS_FALLBACK = '0';
  }

  try {
    const bridge = createOpsLaneBridge(__dirname, 'run_protheus_ops', domain, {
      inheritStdio: true,
      preferLocalCore: true,
    });
    const out = withScopedEnv(envOverrides, () => bridge.run(passArgs));
    if (out && out.stdout) writeAll(1, out.stdout);
    if (out && out.stderr) writeAll(2, out.stderr);
    if (out && out.payload && !out.stdout) {
      writeAll(1, `${JSON.stringify(out.payload)}\n`);
    }
    return Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1;
  } catch {
    return null;
  }
}

function runProtheusOpsLegacy(args, options = {}) {
  const env = { ...process.env, PROTHEUS_ROOT: ROOT, ...(options.env || {}) };
  const bin = resolveBinary({ env });
  if (bin) {
    const proc = spawnInvocation(bin, args, env);
    if (shouldFallbackToCargo(args, proc, options)) {
      const fallback = runViaCargo(args, env);
      if (!fallback.error) {
        emitProcessOutput(fallback);
        return processStatus(fallback);
      }
    }
    emitProcessOutput(proc);
    return processStatus(proc);
  }

  const proc = runViaCargo(args, env);
  emitProcessOutput(proc);
  return processStatus(proc);
}

function runProtheusOps(args, options = {}) {
  if (!legacyProcessRunnerForced(options && options.env ? options.env : process.env)) {
  const viaBridge = runProtheusOpsViaBridge(args, options);
    if (Number.isFinite(Number(viaBridge))) {
      return Number(viaBridge);
    }
  }
  return runProtheusOpsLegacy(args, options);
}

module.exports = { ROOT, resolveBinary, runProtheusOps, runProtheusOpsViaBridge };

if (require.main === module) {
  const exitCode = runProtheusOps(process.argv.slice(2));
  process.exit(exitCode);
}
