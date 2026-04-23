#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../runtime/lib/rust_lane_bridge.ts');

process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'request_envelope', 'request-envelope-kernel');

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
      : (out && out.stderr ? String(out.stderr).trim() : `request_envelope_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `request_envelope_kernel_${command}_failed`);
    return { ok: false, error: message || `request_envelope_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `request_envelope_kernel_${command}_bridge_failed`
      : `request_envelope_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function envelopePayload(input = {}) {
  const out = invoke('envelope-payload', input && typeof input === 'object' ? input : {});
  return out.payload && typeof out.payload === 'object' ? out.payload : null;
}

function canonicalEnvelopeString(payload = {}) {
  const out = invoke('canonical-string', payload && typeof payload === 'object' ? payload : {});
  return String(out.canonical || '');
}

function signEnvelope(payload = {}, secret) {
  const out = invoke('sign', {
    ...(payload && typeof payload === 'object' ? payload : {}),
    secret
  });
  return String(out.signature || '');
}

function verifyEnvelope(input = {}) {
  return invoke('verify', input && typeof input === 'object' ? input : {});
}

function stampGuardEnv(baseEnv = {}, { source = 'local', action = 'apply', files = [], secret, ts, nonce, kid } = {}) {
  const out = invoke('stamp-env', {
    baseEnv: baseEnv && typeof baseEnv === 'object' ? baseEnv : {},
    source,
    action,
    files: Array.isArray(files) ? files : [],
    secret,
    ts,
    nonce,
    kid
  });
  return out.env && typeof out.env === 'object' ? out.env : { ...(baseEnv || {}) };
}

function verifySignedEnvelopeFromEnv({ env = process.env, files = [], secret, maxSkewSec = 900, nowSec } = {}) {
  return invoke('verify-from-env', {
    env: env && typeof env === 'object' ? env : {},
    files: Array.isArray(files) ? files : [],
    secret,
    maxSkewSec,
    nowSec
  });
}

function normalizeFiles(files) {
  const out = invoke('normalize-files', { files: Array.isArray(files) ? files : [] });
  return Array.isArray(out.files) ? out.files : [];
}

function normalizeKeyId(value) {
  const out = invoke('normalize-key-id', { value });
  return String(out.kid || '');
}

function secretKeyEnvVarName(kid) {
  const out = invoke('secret-key-env-var-name', { kid });
  return String(out.env_var || '');
}

module.exports = {
  envelopePayload,
  canonicalEnvelopeString,
  signEnvelope,
  verifyEnvelope,
  stampGuardEnv,
  verifySignedEnvelopeFromEnv,
  normalizeFiles,
  normalizeKeyId,
  secretKeyEnvVarName
};
