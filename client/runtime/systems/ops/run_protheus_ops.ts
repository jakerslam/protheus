#!/usr/bin/env node
'use strict';

// Layer ownership: client/runtime/systems/ops (authoritative app bridge helper)

import fs from 'fs';
import path from 'path';
import { spawnSync, type SpawnSyncReturns } from 'child_process';

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');

function isFile(filePath: string): boolean {
  try {
    return fs.statSync(filePath).isFile();
  } catch {
    return false;
  }
}

function mtimeMs(filePath: string): number {
  try {
    return fs.statSync(filePath).mtimeMs || 0;
  } catch {
    return 0;
  }
}

function sourceNewestMtimeMs(): number {
  const opsRoot = path.join(ROOT, 'core', 'layer0', 'ops');
  const srcRoot = path.join(opsRoot, 'src');
  let newest = Math.max(
    mtimeMs(path.join(ROOT, 'Cargo.toml')),
    mtimeMs(path.join(opsRoot, 'Cargo.toml'))
  );

  const stack: string[] = [srcRoot];
  while (stack.length > 0) {
    const dir = stack.pop() as string;
    let entries: fs.Dirent[] = [];
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

function binaryFreshEnough(binPath: string): boolean {
  const binMtime = mtimeMs(binPath);
  if (!binMtime) return false;
  const srcMtime = sourceNewestMtimeMs();
  if (!srcMtime) return true;
  return binMtime >= srcMtime;
}

function allowStaleBinary(env: NodeJS.ProcessEnv = process.env): boolean {
  const raw = String(
    (env && (env.PROTHEUS_OPS_ALLOW_STALE || env.PROTHEUS_NPM_ALLOW_STALE)) || ''
  )
    .trim()
    .toLowerCase();
  return raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on';
}

type ResolveBinaryOptions = {
  env?: NodeJS.ProcessEnv;
};

function resolveBinary(options: ResolveBinaryOptions = {}): string | null {
  const env = options && options.env ? options.env : process.env;
  const allowStale = allowStaleBinary(env);
  const explicit = String(process.env.PROTHEUS_NPM_BINARY || '').trim();
  if (explicit && isFile(explicit)) return explicit;

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
  if (candidates.length > 0) return candidates[0].binPath;

  return null;
}

function spawnInvocation(
  command: string,
  args: string[],
  env: NodeJS.ProcessEnv
): SpawnSyncReturns<string> {
  return spawnSync(command, args, {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: 'pipe',
    env
  });
}

function processStatus(proc: SpawnSyncReturns<string> | null | undefined): number {
  if (!proc) return 1;
  if (proc.error) return 1;
  return Number.isFinite(proc.status as number) ? (proc.status as number) : 1;
}

function processOutput(proc: SpawnSyncReturns<string> | null | undefined): {
  stdout: string;
  stderr: string;
  combined: string;
} {
  const stdout = proc && typeof proc.stdout === 'string' ? proc.stdout : '';
  const stderrBase = proc && typeof proc.stderr === 'string' ? proc.stderr : '';
  const err = proc && proc.error ? `\n${String(proc.error.message || proc.error)}` : '';
  return {
    stdout,
    stderr: `${stderrBase}${err}`,
    combined: `${stdout}\n${stderrBase}${err}`
  };
}

function writeAll(fd: number, text: string): void {
  if (!text) return;
  const buffer = Buffer.from(text, 'utf8');
  let offset = 0;
  while (offset < buffer.length) {
    offset += fs.writeSync(fd, buffer, offset, buffer.length - offset);
  }
}

function emitProcessOutput(proc: SpawnSyncReturns<string> | null | undefined): void {
  const out = processOutput(proc);
  writeAll(1, out.stdout);
  writeAll(2, out.stderr);
}

type RunOptions = {
  env?: NodeJS.ProcessEnv;
  unknownDomainFallback?: boolean;
};

function shouldFallbackToCargo(
  args: string[],
  proc: SpawnSyncReturns<string>,
  options: RunOptions = {}
): boolean {
  if (options.unknownDomainFallback === false) return false;
  if (!Array.isArray(args) || args.length === 0) return false;
  if (processStatus(proc) === 0) return false;
  const out = processOutput(proc);
  return /\bunknown_domain\b/i.test(out.combined);
}

function runViaCargo(args: string[], env: NodeJS.ProcessEnv): SpawnSyncReturns<string> {
  return spawnInvocation(
    'cargo',
    ['run', '--quiet', '-p', 'protheus-ops-core', '--bin', 'protheus-ops', '--'].concat(args),
    env
  );
}

function runProtheusOps(args: string[], options: RunOptions = {}): number {
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

module.exports = { ROOT, resolveBinary, runProtheusOps };

if (require.main === module) {
  const exitCode = runProtheusOps(process.argv.slice(2));
  process.exit(exitCode);
}
