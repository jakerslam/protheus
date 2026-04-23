#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../../../../client/runtime/lib/rust_lane_bridge.ts');

process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'reflex_store', 'reflex-store-kernel');
const FORBIDDEN_RUNTIME_CONTEXT_MARKERS = [
  'You are an expert Python programmer.',
  '[PATCH v2',
  'List Leaves (25',
  'BEGIN_OPENCLAW_INTERNAL_CONTEXT',
  'END_OPENCLAW_INTERNAL_CONTEXT',
  'UNTRUSTED_CHILD_RESULT_DELIMITER'
];

function containsForbiddenRuntimeContextMarker(raw = '') {
  const text = String(raw);
  return FORBIDDEN_RUNTIME_CONTEXT_MARKERS.some((marker) => text.includes(marker));
}

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function normalizeObject(value) {
  return value && typeof value === 'object' && !Array.isArray(value) ? { ...value } : {};
}

function cloneJsonSafe(value) {
  return JSON.parse(JSON.stringify(value));
}

function invoke(command, payload = {}, opts = {}) {
  const out = bridge.run([
    command,
    `--payload-base64=${encodeBase64(JSON.stringify(normalizeObject(payload)))}`
  ]);
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  const payloadOut = receipt && receipt.payload && typeof receipt.payload === 'object'
    ? receipt.payload
    : receipt;
  if (out.status !== 0) {
    const message = payloadOut && typeof payloadOut.error === 'string'
      ? payloadOut.error
      : (out && out.stderr ? String(out.stderr).trim() : `reflex_store_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `reflex_store_kernel_${command}_failed`);
    return { ok: false, error: message || `reflex_store_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `reflex_store_kernel_${command}_bridge_failed`
      : `reflex_store_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function storePayload(filePath) {
  return filePath ? { file_path: String(filePath) } : {};
}

function defaultReflexState() {
  return invoke('default-state').state;
}

function normalizeState(raw, fallback = null) {
  return invoke('normalize-state', {
    state: normalizeObject(raw),
    fallback: fallback && typeof fallback === 'object' ? fallback : defaultReflexState()
  }).state;
}

function readReflexState(filePath, fallback = null) {
  return invoke('read-state', {
    ...storePayload(filePath),
    fallback: fallback && typeof fallback === 'object' ? fallback : defaultReflexState()
  }).state;
}

function ensureReflexState(filePath, meta = {}) {
  return invoke('ensure-state', {
    ...storePayload(filePath),
    meta: normalizeObject(meta)
  }).state;
}

function setReflexState(filePath, nextState, meta = {}) {
  return invoke('set-state', {
    ...storePayload(filePath),
    state: nextState && typeof nextState === 'object' ? nextState : defaultReflexState(),
    meta: normalizeObject(meta)
  }).state;
}

function mutateReflexState(filePath, mutator, meta = {}) {
  if (typeof mutator !== 'function') throw new Error('reflex_store: mutator must be function');
  const current = readReflexState(filePath, defaultReflexState());
  const base = {
    ...current,
    policy: { ...(current.policy || {}) },
    routines: Array.isArray(current.routines) ? current.routines.map((row) => ({ ...row })) : [],
    metrics: { ...(current.metrics || {}) }
  };
  const next = mutator(cloneJsonSafe(base));
  return setReflexState(filePath, next, {
    ...normalizeObject(meta),
    reason: meta && meta.reason ? meta.reason : 'mutate_reflex_state'
  });
}

module.exports = {
  DEFAULT_REL_PATH: 'reflex/registry.json',
  defaultReflexState,
  normalizeState,
  readReflexState,
  ensureReflexState,
  setReflexState,
  mutateReflexState,
  forbiddenRuntimeContextMarkers: FORBIDDEN_RUNTIME_CONTEXT_MARKERS,
  containsForbiddenRuntimeContextMarker
};

export {};
