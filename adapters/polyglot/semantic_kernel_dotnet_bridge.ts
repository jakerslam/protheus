#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/polyglot (thin interop bridge over semantic-kernel-bridge)

const bridge = require('../../client/runtime/systems/workflow/semantic_kernel_bridge.ts');
const MAX_KEY_LEN = 96;
const MAX_STRING_LEN = 512;
const MAX_ARRAY_ITEMS = 32;

function sanitizeToken(value, maxLen = MAX_STRING_LEN) {
  return String(value == null ? '' : value)
    .replace(/[\u200B\u200C\u200D\u2060\uFEFF]/g, '')
    .replace(/[\r\n\t]+/g, ' ')
    .replace(/[^\x20-\x7E]+/g, '')
    .trim()
    .slice(0, maxLen);
}

function sanitizeValue(value, depth = 0) {
  if (depth > 3) return undefined;
  if (value == null) return undefined;
  if (typeof value === 'string') return sanitizeToken(value, MAX_STRING_LEN);
  if (typeof value === 'number') return Number.isFinite(value) ? value : undefined;
  if (typeof value === 'boolean') return value;
  if (Array.isArray(value)) {
    return value
      .slice(0, MAX_ARRAY_ITEMS)
      .map((entry) => sanitizeValue(entry, depth + 1))
      .filter((entry) => entry !== undefined);
  }
  if (typeof value === 'object') {
    const out = {};
    for (const [rawKey, rawVal] of Object.entries(value)) {
      const key = sanitizeToken(rawKey, MAX_KEY_LEN);
      if (!key) continue;
      const sanitized = sanitizeValue(rawVal, depth + 1);
      if (sanitized !== undefined) {
        out[key] = sanitized;
      }
    }
    return out;
  }
  return undefined;
}

function sanitizePayload(payload = {}) {
  const base = sanitizeValue(payload, 0);
  const normalized = base && typeof base === 'object' && !Array.isArray(base) ? base : {};
  return {
    bridge_path: 'adapters/polyglot/semantic_kernel_dotnet_bridge.ts',
    ...normalized,
  };
}

function callBridge(methodName, payload = {}) {
  const target = bridge && bridge[methodName];
  if (typeof target !== 'function') {
    return {
      ok: false,
      error: `semantic_kernel_dotnet_${methodName}_unavailable`,
      status: 1,
    };
  }
  try {
    return target(sanitizePayload(payload));
  } catch (error) {
    return {
      ok: false,
      error: `semantic_kernel_dotnet_${methodName}_failed`,
      detail: String(error && error.message ? error.message : error || 'unknown_error'),
      status: 1,
    };
  }
}

function registerBridge(payload = {}) {
  return callBridge('registerDotnetBridge', payload);
}

function invokeBridge(payload = {}) {
  return callBridge('invokeDotnetBridge', payload);
}

module.exports = {
  registerBridge,
  invokeBridge,
};
