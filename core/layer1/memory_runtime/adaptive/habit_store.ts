#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../../../../client/runtime/lib/rust_lane_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'habit_store', 'habit-store-kernel');
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
      : (out && out.stderr ? String(out.stderr).trim() : `habit_store_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `habit_store_kernel_${command}_failed`);
    return { ok: false, error: message || `habit_store_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `habit_store_kernel_${command}_bridge_failed`
      : `habit_store_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function storePayload(filePath) {
  return filePath ? { file_path: String(filePath) } : {};
}

function defaultHabitState() {
  return invoke('default-state').state;
}

function normalizeState(raw, fallback = null) {
  return invoke('normalize-state', {
    state: normalizeObject(raw),
    fallback: fallback && typeof fallback === 'object' ? fallback : defaultHabitState()
  }).state;
}

function readHabitState(filePath, fallback = null) {
  return invoke('read-state', {
    ...storePayload(filePath),
    fallback: fallback && typeof fallback === 'object' ? fallback : defaultHabitState()
  }).state;
}

function ensureHabitState(filePath, meta = {}) {
  return invoke('ensure-state', {
    ...storePayload(filePath),
    meta: normalizeObject(meta)
  }).state;
}

function setHabitState(filePath, nextState, meta = {}) {
  return invoke('set-state', {
    ...storePayload(filePath),
    state: nextState && typeof nextState === 'object' ? nextState : defaultHabitState(),
    meta: normalizeObject(meta)
  }).state;
}

function mutateHabitState(filePath, mutator, meta = {}) {
  if (typeof mutator !== 'function') throw new Error('habit_store: mutator must be function');
  const current = readHabitState(filePath, defaultHabitState());
  const base = {
    ...current,
    policy: { ...(current.policy || {}) },
    routines: Array.isArray(current.routines) ? current.routines.map((row) => ({ ...row })) : [],
    metrics: { ...(current.metrics || {}) }
  };
  const next = mutator(cloneJsonSafe(base));
  return setHabitState(filePath, next, {
    ...normalizeObject(meta),
    reason: meta && meta.reason ? meta.reason : 'mutate_habit_state'
  });
}

module.exports = {
  DEFAULT_REL_PATH: 'habits/registry.json',
  defaultHabitState,
  normalizeState,
  readHabitState,
  ensureHabitState,
  setHabitState,
  mutateHabitState,
  forbiddenRuntimeContextMarkers: FORBIDDEN_RUNTIME_CONTEXT_MARKERS,
  containsForbiddenRuntimeContextMarker
};

export {};
