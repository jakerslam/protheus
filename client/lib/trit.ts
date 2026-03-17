#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../runtime/lib/rust_lane_bridge.ts');

const TRIT_PAIN = -1;
const TRIT_UNKNOWN = 0;
const TRIT_OK = 1;

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'trit', 'trit-kernel');

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function invoke(command, payload = {}, opts = {}) {
  const out = bridge.run([
    command,
    `--payload-base64=${encodeBase64(JSON.stringify(payload || {}))}`
  ]);
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  const payloadOut = receipt && receipt.payload && typeof receipt.payload === 'object'
    ? receipt.payload
    : receipt;
  if (out.status !== 0) {
    const message = payloadOut && typeof payloadOut.error === 'string'
      ? payloadOut.error
      : (out && out.stderr ? String(out.stderr).trim() : `trit_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `trit_kernel_${command}_failed`);
    return { ok: false, error: message || `trit_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `trit_kernel_${command}_bridge_failed`
      : `trit_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function normalizeTrit(value) {
  const out = invoke('normalize', { value });
  return Number(out.trit || 0);
}

function tritLabel(value) {
  const out = invoke('label', { value });
  return String(out.label || 'unknown');
}

function tritFromLabel(value) {
  const out = invoke('from-label', { value });
  return Number(out.trit || 0);
}

function invertTrit(value) {
  const out = invoke('invert', { value });
  return Number(out.trit || 0);
}

function majorityTrit(values, opts = {}) {
  const out = invoke('majority', {
    values: Array.isArray(values) ? values : [],
    weights: Array.isArray(opts.weights) ? opts.weights : [],
    tie_breaker: opts.tie_breaker || 'unknown'
  });
  return Number(out.trit || 0);
}

function consensusTrit(values) {
  const out = invoke('consensus', { values: Array.isArray(values) ? values : [] });
  return Number(out.trit || 0);
}

function propagateTrit(parent, child, opts = {}) {
  const out = invoke('propagate', {
    parent,
    child,
    mode: opts.mode || 'cautious'
  });
  return Number(out.trit || 0);
}

function serializeTrit(value) {
  const out = invoke('serialize', { value });
  return String(out.serialized || '0');
}

function parseSerializedTrit(value) {
  const out = invoke('parse-serialized', { value });
  return Number(out.trit || 0);
}

function serializeTritVector(values) {
  const out = invoke('serialize-vector', { values: Array.isArray(values) ? values : [] });
  return out.vector && typeof out.vector === 'object' ? out.vector : null;
}

function parseTritVector(payload) {
  const out = invoke('parse-vector', { payload });
  return Array.isArray(out.values) ? out.values.map((row) => Number(row || 0)) : [];
}

module.exports = {
  TRIT_PAIN,
  TRIT_UNKNOWN,
  TRIT_OK,
  normalizeTrit,
  tritLabel,
  tritFromLabel,
  invertTrit,
  majorityTrit,
  consensusTrit,
  propagateTrit,
  serializeTrit,
  parseSerializedTrit,
  serializeTritVector,
  parseTritVector
};
