#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../runtime/lib/rust_lane_bridge.ts');

process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'proposal_type_classifier', 'proposal-type-classifier-kernel');

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function invoke(command, payload = {}, opts = {}) {
  const out = bridge.run([
    command,
    `--payload-base64=${encodeBase64(JSON.stringify(payload || {}))}`
  ]);
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  const payloadOut = receipt && typeof receipt.payload === 'object' ? receipt.payload : receipt;
  if (out.status !== 0) {
    const message = payloadOut && typeof payloadOut.error === 'string'
      ? payloadOut.error
      : (out && out.stderr ? String(out.stderr).trim() : `proposal_type_classifier_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `proposal_type_classifier_kernel_${command}_failed`);
    return { ok: false, error: message || `proposal_type_classifier_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `proposal_type_classifier_kernel_${command}_bridge_failed`
      : `proposal_type_classifier_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function normalizeTypeKey(v) {
  const out = invoke('normalize-type-key', { value: v });
  return String(out.type_key || '');
}

function extractSourceEyeId(proposal) {
  const out = invoke('extract-source-eye-id', {
    proposal: proposal && typeof proposal === 'object' ? proposal : {}
  });
  return String(out.source_eye || '');
}

function classifyProposalType(proposal, opts = {}) {
  const out = invoke('classify', {
    proposal: proposal && typeof proposal === 'object' ? proposal : {},
    opts: opts && typeof opts === 'object' ? opts : {}
  });
  return out.classification && typeof out.classification === 'object'
    ? out.classification
    : { type: 'local_state_fallback', inferred: true, source: 'infer:proposal_text' };
}

module.exports = {
  classifyProposalType,
  extractSourceEyeId,
  normalizeTypeKey
};
