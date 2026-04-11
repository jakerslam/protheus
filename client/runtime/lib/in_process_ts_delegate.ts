#!/usr/bin/env node
'use strict';

const path = require('node:path');
const { installTsRequireHook } = require('./ts_bootstrap.ts');

function toArgs(argv) {
  return Array.isArray(argv) ? argv.map((value) => String(value)) : [];
}

function resolveModulePath(modulePath, cwd = process.cwd()) {
  return path.isAbsolute(modulePath) ? modulePath : path.resolve(cwd, modulePath);
}

function patchStreamWrite(stream, sink, tee) {
  const original = stream.write.bind(stream);
  return function patchedWrite(chunk, encoding, callback) {
    const text = Buffer.isBuffer(chunk)
      ? chunk.toString(typeof encoding === 'string' ? encoding : 'utf8')
      : String(chunk == null ? '' : chunk);
    sink(text);
    if (tee) return original(chunk, encoding, callback);
    if (typeof encoding === 'function') encoding();
    if (typeof callback === 'function') callback();
    return true;
  };
}

function loadDelegate(modulePathAbs, exportName, fresh) {
  installTsRequireHook();
  if (fresh !== false) delete require.cache[require.resolve(modulePathAbs)];
  const mod = require(modulePathAbs);
  const candidate =
    (exportName && typeof mod?.[exportName] === 'function' && mod[exportName]) ||
    (typeof mod?.run === 'function' && mod.run) ||
    (typeof mod?.main === 'function' && mod.main) ||
    (typeof mod?.default === 'function' && mod.default);
  if (typeof candidate !== 'function') {
    throw new Error(`in_process_ts_delegate_missing_runner:${modulePathAbs}`);
  }
  return candidate;
}

function nextStatus(value) {
  if (Number.isFinite(Number(process.exitCode))) return Number(process.exitCode);
  if (Number.isFinite(Number(value))) return Number(value);
  return 0;
}

function exitSignal(code) {
  const error = new Error(`in_process_ts_delegate_exit:${String(code ?? 0)}`);
  error.__tsDelegateExit = true;
  error.status = Number.isFinite(Number(code)) ? Number(code) : 0;
  return error;
}

function restoreCwd(previousCwd, activeCwd) {
  if (activeCwd !== previousCwd) process.chdir(previousCwd);
}

function invokeTsModuleSync(modulePath, options = {}) {
  const modulePathAbs = resolveModulePath(modulePath, options.cwd || process.cwd());
  const argv = toArgs(options.argv);
  const activeCwd = path.resolve(options.cwd || path.dirname(modulePathAbs));
  const previousCwd = process.cwd();
  const previousArgv = process.argv.slice();
  const previousExit = process.exit;
  const previousExitCode = process.exitCode;
  const originalStdoutWrite = process.stdout.write.bind(process.stdout);
  const originalStderrWrite = process.stderr.write.bind(process.stderr);
  let stdout = '';
  let stderr = '';
  let value;
  let status = 0;

  if (activeCwd !== previousCwd) process.chdir(activeCwd);
  process.argv = [process.execPath, modulePathAbs, ...argv];
  process.stdout.write = patchStreamWrite(
    process.stdout,
    (chunk) => {
      stdout += chunk;
    },
    options.teeStdout === true,
  );
  process.stderr.write = patchStreamWrite(
    process.stderr,
    (chunk) => {
      stderr += chunk;
    },
    options.teeStderr === true,
  );
  process.exitCode = undefined;
  process.exit = (code) => {
    throw exitSignal(code);
  };

  try {
    const delegate = loadDelegate(modulePathAbs, options.exportName, options.fresh !== false);
    value = delegate(argv);
    if (value && typeof value.then === 'function') {
      throw new Error(`in_process_ts_delegate_async_runner_requires_async_api:${modulePathAbs}`);
    }
    status = nextStatus(value);
  } catch (error) {
    if (error && error.__tsDelegateExit) {
      status = Number.isFinite(Number(error.status)) ? Number(error.status) : 0;
    } else {
      throw error;
    }
  } finally {
    process.stdout.write = originalStdoutWrite;
    process.stderr.write = originalStderrWrite;
    process.exit = previousExit;
    process.exitCode = previousExitCode;
    process.argv = previousArgv;
    restoreCwd(previousCwd, activeCwd);
  }

  return { status, stdout, stderr, value };
}

async function invokeTsModuleAsync(modulePath, options = {}) {
  const modulePathAbs = resolveModulePath(modulePath, options.cwd || process.cwd());
  const argv = toArgs(options.argv);
  const activeCwd = path.resolve(options.cwd || path.dirname(modulePathAbs));
  const previousCwd = process.cwd();
  const previousArgv = process.argv.slice();
  const previousExit = process.exit;
  const previousExitCode = process.exitCode;
  const originalStdoutWrite = process.stdout.write.bind(process.stdout);
  const originalStderrWrite = process.stderr.write.bind(process.stderr);
  let stdout = '';
  let stderr = '';
  let value;
  let status = 0;

  if (activeCwd !== previousCwd) process.chdir(activeCwd);
  process.argv = [process.execPath, modulePathAbs, ...argv];
  process.stdout.write = patchStreamWrite(
    process.stdout,
    (chunk) => {
      stdout += chunk;
    },
    options.teeStdout === true,
  );
  process.stderr.write = patchStreamWrite(
    process.stderr,
    (chunk) => {
      stderr += chunk;
    },
    options.teeStderr === true,
  );
  process.exitCode = undefined;
  process.exit = (code) => {
    throw exitSignal(code);
  };

  try {
    const delegate = loadDelegate(modulePathAbs, options.exportName, options.fresh !== false);
    value = await delegate(argv);
    status = nextStatus(value);
  } catch (error) {
    if (error && error.__tsDelegateExit) {
      status = Number.isFinite(Number(error.status)) ? Number(error.status) : 0;
    } else {
      throw error;
    }
  } finally {
    process.stdout.write = originalStdoutWrite;
    process.stderr.write = originalStderrWrite;
    process.exit = previousExit;
    process.exitCode = previousExitCode;
    process.argv = previousArgv;
    restoreCwd(previousCwd, activeCwd);
  }

  return { status, stdout, stderr, value };
}

module.exports = {
  invokeTsModuleSync,
  invokeTsModuleAsync,
};
