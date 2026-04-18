#!/usr/bin/env node
'use strict';

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');
const { normalizeOpsBridgeEnvAliases } = require('./queued_backlog_runtime.ts');

normalizeOpsBridgeEnvAliases();
process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_USE_PREBUILT =
  process.env.PROTHEUS_OPS_USE_PREBUILT || process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';

function normalizeLaneId(raw, fallback = 'RUNTIME-LEGACY-RETIRED') {
  const v = String(raw || '')
    .toUpperCase()
    .replace(/[^A-Z0-9_.-]+/g, '-')
    .replace(/^-+|-+$/g, '');
  return v || fallback;
}

function laneIdFromRuntimePath(filePath) {
  const path = require('path');
  const runtimeRoot = path.resolve(__dirname, '..');
  const rel = path
    .relative(runtimeRoot, filePath)
    .replace(/\\/g, '/')
    .replace(/\.[^.]+$/, '');
  return normalizeLaneId(`RUNTIME-${rel}`);
}

function mapArgs(args = []) {
  const cmd = String((Array.isArray(args) && args[0]) || '').trim().toLowerCase();
  if (!cmd || cmd === 'run') return ['run'];
  if (cmd === 'status' || cmd === 'verify') return ['status'];
  return args.map((v) => String(v));
}

function createLegacyRetiredModule(scriptDir, scriptName, laneId) {
  const bridge = createOpsLaneBridge(scriptDir, scriptName, 'runtime-systems');
  const normalized = normalizeLaneId(laneId);

  function run(args = []) {
    const pass = mapArgs(Array.isArray(args) ? args : []);
    const out = bridge.run([`--lane-id=${normalized}`].concat(pass));
    if (out && out.stdout) process.stdout.write(out.stdout);
    if (out && out.stderr) process.stderr.write(out.stderr);
    if (out && out.payload && !out.stdout) {
      process.stdout.write(`${JSON.stringify(out.payload)}\n`);
    }
    return out;
  }

  return {
    lane: bridge.lane,
    run
  };
}

function runAsMain(mod, argv = []) {
  function maybePrintPayload(out) {
    if (
      out
      && typeof out === 'object'
      && !Array.isArray(out)
      && typeof out.stdout !== 'string'
      && typeof out.stderr !== 'string'
    ) {
      process.stdout.write(`${JSON.stringify(out)}\n`);
    }
  }

  function exitFromResult(out) {
    maybePrintPayload(out);
    const exitCode =
      typeof out === 'number'
        ? out
        : (out && out.status);
    process.exit(Number.isFinite(Number(exitCode)) ? Number(exitCode) : 1);
  }

  try {
    const out = mod.run(argv);
    if (out && typeof out.then === 'function') {
      out.then(exitFromResult).catch((err) => {
        const payload = {
          ok: false,
          error: String((err && (err.code || err.message)) || err || 'compatibility_bridge_failed')
        };
        process.stderr.write(`${JSON.stringify(payload)}\n`);
        process.exit(1);
      });
      return;
    }
    exitFromResult(out);
  } catch (err) {
    const payload = {
      ok: false,
      error: String((err && (err.code || err.message)) || err || 'compatibility_bridge_failed')
    };
    process.stderr.write(`${JSON.stringify(payload)}\n`);
    process.exit(1);
  }
}

function createLegacyRetiredModuleForFile(filePath) {
  const path = require('path');
  const laneId = laneIdFromRuntimePath(filePath);
  return createLegacyRetiredModule(path.dirname(filePath), path.basename(filePath), laneId);
}

function bindLegacyRetiredModule(filePath, currentModule, argv = process.argv.slice(2)) {
  const mod = createLegacyRetiredModuleForFile(filePath);
  if (currentModule && require.main === currentModule) runAsMain(mod, argv);
  return mod;
}

function formatBridgeErrorDetail(error) {
  return String((error && (error.message || error.code)) || error || 'unknown_error');
}

function loadBoundModuleFromRuntime(
  runtime,
  filePath,
  currentModule,
  missingBindError = 'legacy_retired_target_missing_bind',
  loadFailedError = 'legacy_retired_target_load_failed'
) {
  try {
    if (!runtime || typeof runtime.bindLegacyRetiredModule !== 'function') {
      return { ok: false, error: String(missingBindError || 'legacy_retired_target_missing_bind') };
    }
    return runtime.bindLegacyRetiredModule(filePath, currentModule);
  } catch (error) {
    return {
      ok: false,
      error: String(loadFailedError || 'legacy_retired_target_load_failed'),
      detail: formatBridgeErrorDetail(error)
    };
  }
}

function exitIfBoundModuleFailed(bound, currentModule) {
  if (currentModule && require.main === currentModule && bound && bound.ok === false) {
    process.stderr.write(`${JSON.stringify(bound)}\n`);
    process.exit(1);
  }
}

function bindLegacyRetiredModuleSafe(
  filePath,
  currentModule,
  missingBindError = 'legacy_retired_target_missing_bind',
  loadFailedError = 'legacy_retired_target_load_failed'
) {
  const runtime = module.exports;
  const bound = loadBoundModuleFromRuntime(
    runtime,
    filePath,
    currentModule,
    missingBindError,
    loadFailedError
  );
  exitIfBoundModuleFailed(bound, currentModule);
  return bound;
}

function createCompatibilityBridgeModule(implPath) {
  const impl = require(implPath);

  function run(args = process.argv.slice(2)) {
    return impl.run(Array.isArray(args) ? args : []);
  }

  return {
    ...impl,
    run
  };
}

function resolveCompatibilityImplPath(implPath, currentModule) {
  const path = require('path');
  if (path.isAbsolute(implPath)) return implPath;
  const callerDir =
    currentModule && currentModule.filename
      ? path.dirname(currentModule.filename)
      : __dirname;
  return path.resolve(callerDir, implPath);
}

function bindCompatibilityBridgeModule(
  implPath,
  currentModule,
  argv = process.argv.slice(2)
) {
  const mod = createCompatibilityBridgeModule(
    resolveCompatibilityImplPath(implPath, currentModule)
  );
  if (currentModule && require.main === currentModule) runAsMain(mod, argv);
  return mod;
}

module.exports = {
  bindCompatibilityBridgeModule,
  bindLegacyRetiredModule,
  bindLegacyRetiredModuleSafe,
  exitIfBoundModuleFailed,
  loadBoundModuleFromRuntime,
  createCompatibilityBridgeModule,
  createLegacyRetiredModuleForFile,
  createLegacyRetiredModule,
  laneIdFromRuntimePath,
  normalizeLaneId,
  runAsMain
};
