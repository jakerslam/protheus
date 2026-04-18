'use strict';

const crypto = require('node:crypto');
const { loadAdaptiveMemoryModule } = require('../../../adapters/runtime/adaptive_memory_bridge.ts');

const DEFAULT_MAX_ARG_LEN = 512;
const DEFAULT_MAX_ARGS = 64;

function formatErrorDetail(error) {
  return String(error && error.message ? error.message : error || 'unknown_error');
}

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

function sanitizeAdaptiveArg(value, maxArgLen = DEFAULT_MAX_ARG_LEN) {
  return String(value == null ? '' : value)
    .replace(/[\u200B\u200C\u200D\u2060\uFEFF]/g, '')
    .replace(/[\r\n\t]+/g, ' ')
    .replace(/[^\x20-\x7E]+/g, '')
    .trim()
    .slice(0, maxArgLen);
}

function resolvePositiveIntegerOption(value, fallback, min, max) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return fallback;
  }
  const normalized = Math.trunc(parsed);
  if (normalized < min || normalized > max) {
    return fallback;
  }
  return normalized;
}

function normalizeArgs(args, maxArgLen, maxArgs) {
  return Array.isArray(args)
    ? args.map((arg) => sanitizeAdaptiveArg(arg, maxArgLen)).filter(Boolean).slice(0, maxArgs)
    : [];
}

function createAdaptiveMemoryEntrypoint(moduleId, options = {}) {
  const MODULE_ID = String(moduleId || '').trim();
  const MAX_ARG_LEN = resolvePositiveIntegerOption(
    options.maxArgLen,
    DEFAULT_MAX_ARG_LEN,
    16,
    4096
  );
  const MAX_ARGS = resolvePositiveIntegerOption(options.maxArgs, DEFAULT_MAX_ARGS, 1, 256);

  function loadTarget() {
    try {
      return loadAdaptiveMemoryModule(MODULE_ID);
    } catch (error) {
      return withReceiptHash({
        ok: false,
        error: `${MODULE_ID}_target_load_failed`,
        module_id: MODULE_ID,
        detail: formatErrorDetail(error)
      });
    }
  }

  const target = loadTarget();

  function run(args = process.argv.slice(2)) {
    if (!target || target.ok === false) {
      process.stderr.write(
        `${JSON.stringify(
          withReceiptHash(target || { ok: false, error: `${MODULE_ID}_target_unavailable`, module_id: MODULE_ID })
        )}\n`
      );
      return 1;
    }
    if (typeof target.run !== 'function') {
      process.stderr.write(
        `${JSON.stringify(
          withReceiptHash({ ok: false, error: `${MODULE_ID}_target_missing_run`, module_id: MODULE_ID })
        )}\n`
      );
      return 1;
    }
    return target.run(normalizeArgs(args, MAX_ARG_LEN, MAX_ARGS));
  }

  function runAsMain(args = process.argv.slice(2)) {
    const code = run(args);
    process.exit(Number.isFinite(Number(code)) ? Number(code) : 1);
  }

  return {
    moduleId: MODULE_ID,
    target,
    run,
    runAsMain,
    normalizeReceiptHash
  };
}

module.exports = {
  DEFAULT_MAX_ARGS,
  DEFAULT_MAX_ARG_LEN,
  sanitizeAdaptiveArg,
  createAdaptiveMemoryEntrypoint,
  normalizeReceiptHash
};
