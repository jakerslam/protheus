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

const DEFAULT_REL_PATH = 'sensory/eyes/catalog.json';
const DEFAULT_ABS_PATH = path.join(runtimeRoot(), 'adaptive', DEFAULT_REL_PATH);

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'catalog_store', 'catalog-store-kernel');

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
      : (out && out.stderr ? String(out.stderr).trim() : `catalog_store_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `catalog_store_kernel_${command}_failed`);
    return { ok: false, error: message || `catalog_store_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `catalog_store_kernel_${command}_bridge_failed`
      : `catalog_store_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function storePayload(filePath) {
  if (!filePath) return {};
  return { file_path: String(filePath) };
}

function defaultCatalog() {
  return invoke('default-state').state;
}

function normalizeCatalog(raw, fallback = null) {
  return invoke('normalize-state', {
    state: raw && typeof raw === 'object' ? raw : fallback && typeof fallback === 'object' ? fallback : defaultCatalog()
  }).state;
}

function readCatalog(filePath, fallback = null) {
  return invoke('read-state', {
    ...storePayload(filePath),
    fallback: fallback && typeof fallback === 'object' ? fallback : defaultCatalog()
  }).state;
}

function ensureCatalog(filePath, meta = {}) {
  return invoke('ensure-state', {
    ...storePayload(filePath),
    meta: normalizeObject(meta)
  }).state;
}

function setCatalog(filePath, nextCatalog, meta = {}) {
  return invoke('set-state', {
    ...storePayload(filePath),
    state: nextCatalog && typeof nextCatalog === 'object' ? nextCatalog : defaultCatalog(),
    meta: normalizeObject(meta)
  }).state;
}

function mutateCatalog(filePath, mutator, meta = {}) {
  if (typeof mutator !== 'function') throw new Error('catalog_store: mutator must be function');
  const current = readCatalog(filePath, defaultCatalog());
  const base = cloneJsonSafe(current || {});
  const next = mutator(base);
  return setCatalog(filePath, next, {
    ...normalizeObject(meta),
    reason: meta && meta.reason ? meta.reason : 'mutate_catalog'
  });
}

module.exports = {
  DEFAULT_REL_PATH,
  DEFAULT_ABS_PATH,
  defaultCatalog,
  readCatalog,
  ensureCatalog,
  setCatalog,
  mutateCatalog
};

export {};
