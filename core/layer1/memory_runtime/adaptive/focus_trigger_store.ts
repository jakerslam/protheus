#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const path = require('path');
const { createOpsLaneBridge } = require('../../../../client/runtime/lib/rust_lane_bridge.ts');

function workspaceRoot() {
  return process.env.PROTHEUS_WORKSPACE_ROOT
    ? path.resolve(String(process.env.PROTHEUS_WORKSPACE_ROOT))
    : path.resolve(__dirname, '..', '..', '..', '..');
}

function runtimeRoot() {
  return process.env.PROTHEUS_RUNTIME_ROOT
    ? path.resolve(String(process.env.PROTHEUS_RUNTIME_ROOT))
    : path.join(workspaceRoot(), 'client', 'runtime');
}

const DEFAULT_REL_PATH = 'sensory/eyes/focus_triggers.json';
const DEFAULT_ABS_PATH = path.join(runtimeRoot(), 'adaptive', DEFAULT_REL_PATH);

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'focus_trigger_store', 'focus-trigger-store-kernel');
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
      : (out && out.stderr ? String(out.stderr).trim() : `focus_trigger_store_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `focus_trigger_store_kernel_${command}_failed`);
    return { ok: false, error: message || `focus_trigger_store_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `focus_trigger_store_kernel_${command}_bridge_failed`
      : `focus_trigger_store_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function storePayload(filePath) {
  if (!filePath) return {};
  return { file_path: String(filePath) };
}

function defaultFocusState() {
  return invoke('default-state').state;
}

function normalizeState(raw, fallback = null) {
  return invoke('normalize-state', {
    state: normalizeObject(raw),
    fallback: fallback && typeof fallback === 'object' ? fallback : defaultFocusState()
  }).state;
}

function readFocusState(filePath, fallback = null) {
  return invoke('read-state', {
    ...storePayload(filePath),
    fallback: fallback && typeof fallback === 'object' ? fallback : defaultFocusState()
  }).state;
}

function ensureFocusState(filePath, meta = {}) {
  return invoke('ensure-state', {
    ...storePayload(filePath),
    meta: normalizeObject(meta)
  }).state;
}

function setFocusState(filePath, nextState, meta = {}) {
  return invoke('set-state', {
    ...storePayload(filePath),
    state: nextState && typeof nextState === 'object' ? nextState : defaultFocusState(),
    meta: normalizeObject(meta)
  }).state;
}

function mutateFocusState(filePath, mutator, meta = {}) {
  if (typeof mutator !== 'function') throw new Error('focus_trigger_store: mutator must be function');
  const current = readFocusState(filePath, defaultFocusState());
  const base = {
    ...current,
    policy: { ...(current.policy || {}) },
    triggers: Array.isArray(current.triggers) ? current.triggers.map((row) => ({ ...row })) : [],
    eye_lenses: cloneJsonSafe(current.eye_lenses || {}),
    recent_focus_items: { ...(current.recent_focus_items || {}) },
    stats: { ...(current.stats || {}) },
    last_refresh_sources: { ...(current.last_refresh_sources || {}) },
    last_lens_refresh_sources: { ...(current.last_lens_refresh_sources || {}) }
  };
  const next = mutator(base);
  return setFocusState(filePath, next, {
    ...normalizeObject(meta),
    reason: meta && meta.reason ? meta.reason : 'mutate_focus_state'
  });
}

module.exports = {
  DEFAULT_REL_PATH,
  DEFAULT_ABS_PATH,
  defaultFocusState,
  normalizeState,
  readFocusState,
  ensureFocusState,
  setFocusState,
  mutateFocusState,
  forbiddenRuntimeContextMarkers: FORBIDDEN_RUNTIME_CONTEXT_MARKERS,
  containsForbiddenRuntimeContextMarker
};

export {};
