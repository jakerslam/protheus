#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');
const { normalizeOpsBridgeEnvAliases } = require('./queued_backlog_runtime.ts');

normalizeOpsBridgeEnvAliases();
process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_USE_PREBUILT =
  process.env.PROTHEUS_OPS_USE_PREBUILT || process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'runtime_path_registry', 'runtime-path-registry-kernel');

function encodeBase64(value: unknown) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function invoke(command: string, payload: Record<string, unknown> = {}, opts: Record<string, unknown> = {}) {
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
      : (out && out.stderr ? String(out.stderr).trim() : `runtime_path_registry_kernel_${command}_failed`);
    if (opts && opts.throwOnError === false) return { ok: false, error: message || `runtime_path_registry_kernel_${command}_failed` };
    throw new Error(message || `runtime_path_registry_kernel_${command}_failed`);
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `runtime_path_registry_kernel_${command}_bridge_failed`
      : `runtime_path_registry_kernel_${command}_bridge_failed`;
    if (opts && opts.throwOnError === false) return { ok: false, error: message };
    throw new Error(message);
  }
  return payloadOut;
}

const constants = invoke('constants', {});
const CANONICAL_PATHS = constants.canonical_paths || {};
const LEGACY_SURFACES = Array.isArray(constants.legacy_surfaces) ? constants.legacy_surfaces : [];

function normalizeForRoot(rootAbs: string, relPath: string) {
  return String(invoke('normalize-for-root', { root_abs: rootAbs, rel_path: relPath }).value || '');
}

function resolveCanonical(rootAbs: string, relPath: string) {
  return String(invoke('resolve-canonical', { root_abs: rootAbs, rel_path: relPath }).value || '');
}

function resolveClientState(rootAbs: string, suffix = '') {
  return String(invoke('resolve-client-state', { root_abs: rootAbs, suffix }).value || '');
}

function resolveCoreState(rootAbs: string, suffix = '') {
  return String(invoke('resolve-core-state', { root_abs: rootAbs, suffix }).value || '');
}

module.exports = {
  CANONICAL_PATHS,
  LEGACY_SURFACES,
  normalizeForRoot,
  resolveCanonical,
  resolveClientState,
  resolveCoreState
};
