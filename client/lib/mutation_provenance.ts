#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../runtime/lib/rust_lane_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'mutation_provenance', 'mutation-provenance-kernel');

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
      : (out && out.stderr ? String(out.stderr).trim() : `mutation_provenance_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `mutation_provenance_kernel_${command}_failed`);
    return { ok: false, error: message || `mutation_provenance_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `mutation_provenance_kernel_${command}_bridge_failed`
      : `mutation_provenance_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function normalizeMeta(meta, fallbackSource = '', defaultReason = '') {
  const out = invoke('normalize-meta', {
    meta: meta && typeof meta === 'object' ? meta : {},
    fallback_source: String(fallbackSource || ''),
    default_reason: String(defaultReason || '')
  });
  return out.meta && typeof out.meta === 'object' ? out.meta : {};
}

function loadPolicy() {
  const out = invoke('load-policy', {});
  return out.policy && typeof out.policy === 'object' ? out.policy : {};
}

function enforceMutationProvenance(channel, meta, opts = {}) {
  return invoke('enforce', {
    channel: String(channel || ''),
    meta: meta && typeof meta === 'object' ? meta : {},
    fallback_source: opts && typeof opts === 'object' ? String(opts.fallbackSource || '') : '',
    default_reason: opts && typeof opts === 'object' ? String(opts.defaultReason || '') : '',
    opts: opts && typeof opts === 'object' ? opts : {}
  });
}

function recordMutationAudit(channel, row = {}) {
  invoke('record-audit', {
    channel: String(channel || ''),
    row: row && typeof row === 'object' ? row : {}
  });
}

module.exports = {
  normalizeMeta,
  loadPolicy,
  enforceMutationProvenance,
  recordMutationAudit
};
