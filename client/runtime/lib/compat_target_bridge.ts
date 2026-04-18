'use strict';

const crypto = require('node:crypto');
const path = require('path');

const DEFAULT_MAX_ARGS = 64;
const DEFAULT_MAX_ARG_LEN = 512;

function stableStringify(value) {
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map((item) => stableStringify(item)).join(',')}]`;
  }
  const keys = Object.keys(value).sort();
  return `{${keys.map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`).join(',')}}`;
}

function normalizeReceiptHash(payload) {
  const clone = Object.assign({}, payload);
  delete clone.receipt_hash;
  return crypto.createHash('sha256').update(stableStringify(clone)).digest('hex');
}

function withReceiptHash(payload) {
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
    return payload;
  }
  if (typeof payload.receipt_hash === 'string' && payload.receipt_hash.trim()) {
    return payload;
  }
  return Object.assign({}, payload, { receipt_hash: normalizeReceiptHash(payload) });
}

function formatErrorDetail(error) {
  return String(error && error.message ? error.message : error || 'unknown_error');
}

function sanitizeCompatArg(value, maxArgLen = DEFAULT_MAX_ARG_LEN) {
  return String(value == null ? '' : value)
    .replace(/[\u200B\u200C\u200D\u2060\uFEFF]/g, '')
    .replace(/[\r\n\t]+/g, ' ')
    .replace(/[^\x20-\x7E]+/g, '')
    .trim()
    .slice(0, maxArgLen);
}

function resolvePositiveIntegerOption(value, fallback, min = 1, max = 4096) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return fallback;
  const normalized = Math.trunc(parsed);
  if (normalized < min || normalized > max) return fallback;
  return normalized;
}

function normalizeArgs(args, maxArgLen = DEFAULT_MAX_ARG_LEN, maxArgs = DEFAULT_MAX_ARGS) {
  return Array.isArray(args)
    ? args.map((arg) => sanitizeCompatArg(arg, maxArgLen)).filter(Boolean).slice(0, maxArgs)
    : [];
}

function loadTargetModule(targetPath, loadError) {
  try {
    return require(targetPath);
  } catch (error) {
    return withReceiptHash({
      ok: false,
      error: loadError,
      target: targetPath,
      detail: formatErrorDetail(error)
    });
  }
}

function selectTargetExport(target, targetExport, missingExportError, targetPath) {
  if (!target || target.ok === false) return target;
  if (!targetExport) return target;
  if (typeof target !== 'object' || target[targetExport] == null) {
    return withReceiptHash({
      ok: false,
      error: missingExportError,
      target: targetPath
    });
  }
  return target[targetExport];
}

function createCompatModuleExportBridge(options = {}) {
  const scriptDir = String(options.scriptDir || '');
  const targetRelativePath = String(options.targetRelativePath || '');
  const loadError = String(options.loadError || 'compat_target_load_failed');
  const invalidError = String(options.invalidError || 'compat_target_invalid');

  const targetPath = path.resolve(scriptDir, targetRelativePath);
  const target = loadTargetModule(targetPath, loadError);
  const exported =
    target && target.ok !== false && typeof target === 'object' && !Array.isArray(target)
      ? target
      : (target && target.ok === false
          ? target
          : withReceiptHash({ ok: false, error: invalidError, target: targetPath }));

  function exitIfMain(currentModule) {
    if (currentModule && require.main === currentModule && exported && exported.ok === false) {
      process.stderr.write(`${JSON.stringify(exported)}\n`);
      process.exit(1);
    }
  }

  return {
    exported,
    exitIfMain
  };
}

function createCompatWorkflowExportBridge(options = {}) {
  const bridgePath = String(options.bridgePath || '');
  const bridgeTarget = String(options.bridgeTarget || '');
  const framework = String(options.framework || '');
  const bridge = createCompatModuleExportBridge(options);

  function withBridgeMetadata(payload = {}) {
    const base = payload && typeof payload === 'object' && !Array.isArray(payload) ? payload : {};
    return {
      ...(bridgePath ? { bridge_path: bridgePath } : {}),
      ...(bridgeTarget ? { bridge_target: bridgeTarget } : {}),
      ...(framework ? { framework } : {}),
      ...base
    };
  }

  const impl = bridge.exported;
  const exported =
    impl && impl.ok !== false && typeof impl === 'object' && !Array.isArray(impl)
      ? {
          ...(bridgePath ? { BRIDGE_PATH: bridgePath } : {}),
          ...(bridgeTarget ? { BRIDGE_TARGET: bridgeTarget } : {}),
          ...(framework ? { FRAMEWORK: framework } : {}),
          withBridgeMetadata,
          ...impl
        }
      : impl;

  return {
    exported,
    exitIfMain: bridge.exitIfMain
  };
}

function createCompatTargetBridge(options = {}) {
  const scriptDir = String(options.scriptDir || '');
  const targetRelativePath = String(options.targetRelativePath || '');
  const loadError = String(options.loadError || 'compat_target_load_failed');
  const unavailableError = String(options.unavailableError || 'compat_target_unavailable');
  const targetExport = String(options.targetExport || '').trim();
  const missingExportError = String(options.missingExportError || 'compat_target_missing_export');
  const missingRunError = String(options.missingRunError || 'compat_target_missing_run');
  const maxArgs = resolvePositiveIntegerOption(options.maxArgs, DEFAULT_MAX_ARGS, 1, 256);
  const maxArgLen = resolvePositiveIntegerOption(options.maxArgLen, DEFAULT_MAX_ARG_LEN, 16, 4096);

  const targetPath = path.resolve(scriptDir, targetRelativePath);
  const loaded = loadTargetModule(targetPath, loadError);
  const target = selectTargetExport(loaded, targetExport, missingExportError, targetPath);

  function run(args = process.argv.slice(2)) {
    if (!target || target.ok === false) {
      process.stderr.write(
        `${JSON.stringify(
          withReceiptHash(target || { ok: false, error: unavailableError, target: targetPath })
        )}\n`
      );
      return 1;
    }
    if (typeof target.run !== 'function') {
      process.stderr.write(
        `${JSON.stringify(
          withReceiptHash({ ok: false, error: missingRunError, target: targetPath })
        )}\n`
      );
      return 1;
    }
    return target.run(normalizeArgs(args, maxArgLen, maxArgs));
  }

  function runAsMain(args = process.argv.slice(2)) {
    const code = run(args);
    process.exit(Number.isFinite(Number(code)) ? Number(code) : 1);
  }

  return {
    target,
    run,
    runAsMain,
    normalizeReceiptHash,
    withReceiptHash,
    sanitizeCompatArg
  };
}

module.exports = {
  createCompatModuleExportBridge,
  createCompatWorkflowExportBridge,
  createCompatTargetBridge,
  DEFAULT_MAX_ARGS,
  DEFAULT_MAX_ARG_LEN,
  normalizeReceiptHash,
  withReceiptHash,
  sanitizeCompatArg
};
