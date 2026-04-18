#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/polyglot (thin interop bridge over semantic-kernel-bridge)

const crypto = require('node:crypto');
const bridge = require('../../client/runtime/systems/workflow/semantic_kernel_bridge.ts');
const { sanitizeBridgeArg, normalizeReceiptHash } = require('../../client/runtime/lib/runtime_system_entrypoint.ts');

const MAX_KEY_LEN = 96;
const MAX_STRING_LEN = 512;
const MAX_ARRAY_ITEMS = 32;
const MAX_FIELDS = 128;

function sanitizeToken(value, maxLen = MAX_STRING_LEN) {
  return sanitizeBridgeArg(value, maxLen);
}

function stableStringify(value) {
  if (value === null || typeof value !== 'object') return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map((item) => stableStringify(item)).join(',')}]`;
  const keys = Object.keys(value).sort();
  return `{${keys.map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`).join(',')}}`;
}

function attemptSignature(methodName, payload) {
  return crypto
    .createHash('sha256')
    .update(stableStringify({ method: sanitizeToken(methodName, 64), payload }))
    .digest('hex');
}

function classifyError(detail) {
  const text = String(detail || '').toLowerCase();
  if (/timeout|timed\s*out|deadline/.test(text)) return 'timeout';
  if (/enoent|not\s+found|missing|unavailable/.test(text)) return 'transport_unavailable';
  if (/rate\s*limit|429|temporar/.test(text)) return 'transient';
  if (!text) return 'none';
  return 'execution_error';
}

function retryForClass(errorClass) {
  if (errorClass === 'timeout' || errorClass === 'transient') {
    return {
      recommended: true,
      strategy: 'bounded_backoff',
      lane: 'same_lane_retry',
      attempts: 2,
      min_delay_ms: 400,
      max_delay_ms: 5000,
      jitter: 0.1,
    };
  }
  if (errorClass === 'transport_unavailable') {
    return {
      recommended: false,
      strategy: 'manual_recovery',
      lane: 'operator_fix',
    };
  }
  return {
    recommended: false,
    strategy: 'none',
    lane: 'none',
  };
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
    let kept = 0;
    for (const [rawKey, rawVal] of Object.entries(value)) {
      if (kept >= MAX_FIELDS) break;
      const key = sanitizeToken(rawKey, MAX_KEY_LEN);
      if (!key) continue;
      const sanitized = sanitizeValue(rawVal, depth + 1);
      if (sanitized !== undefined) {
        out[key] = sanitized;
        kept += 1;
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

function annotate(payload, context) {
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
    return payload;
  }
  const merged = {
    attempt_signature: context.signature,
    ...payload,
  };
  if (merged.ok === false || context.errorClass !== 'none') {
    if (!merged.error_class) merged.error_class = context.errorClass;
    if (!merged.retry) merged.retry = retryForClass(context.errorClass);
    if (!merged.tool_error_summary) {
      merged.tool_error_summary = {
        toolName: context.toolName,
        meta: context.errorClass,
        error: context.detail || undefined,
        timedOut: context.errorClass === 'timeout',
        actionFingerprint: context.signature,
      };
    }
  }
  if (!merged.receipt_hash) {
    merged.receipt_hash = normalizeReceiptHash(merged);
  }
  return merged;
}

function callBridge(methodName, payload = {}) {
  const sanitizedPayload = sanitizePayload(payload);
  const signature = attemptSignature(methodName, sanitizedPayload);
  const target = bridge && bridge[methodName];
  if (typeof target !== 'function') {
    const detail = `semantic_kernel_dotnet_${methodName}_unavailable`;
    return annotate(
      {
        ok: false,
        error: detail,
        status: 1,
      },
      {
        toolName: `semantic_kernel_dotnet_${methodName}`,
        errorClass: 'transport_unavailable',
        detail,
        signature,
      }
    );
  }
  try {
    return annotate(target(sanitizedPayload), {
      toolName: `semantic_kernel_dotnet_${methodName}`,
      errorClass: 'none',
      detail: '',
      signature,
    });
  } catch (error) {
    const detail = String(error && error.message ? error.message : error || 'unknown_error');
    const errorClass = classifyError(detail);
    return annotate(
      {
        ok: false,
        error: `semantic_kernel_dotnet_${methodName}_failed`,
        detail,
        status: 1,
      },
      {
        toolName: `semantic_kernel_dotnet_${methodName}`,
        errorClass,
        detail,
        signature,
      }
    );
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
