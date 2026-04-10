#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'conversation_eye_synthesizer', 'conversation-eye-synthesizer-kernel');

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function bridgePayload(out) {
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  return receipt && receipt.payload && typeof receipt.payload === 'object'
    ? receipt.payload
    : receipt;
}

function invokeError(command, out, payloadOut, suffix) {
  const fallback = `conversation_eye_synthesizer_kernel_${command}_${suffix}`;
  if (payloadOut && typeof payloadOut.error === 'string' && payloadOut.error.trim()) {
    return payloadOut.error.trim();
  }
  const stderr = out && out.stderr ? String(out.stderr).trim() : '';
  return stderr || fallback;
}

function invoke(command, payload = {}, opts = {}) {
  const out = bridge.run([
    command,
    `--payload-base64=${encodeBase64(JSON.stringify(payload || {}))}`
  ]);
  const payloadOut = bridgePayload(out);
  if (out.status !== 0) {
    const message = invokeError(command, out, payloadOut, 'failed');
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = invokeError(command, out, payloadOut, 'bridge_failed');
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function synthesizeEnvelope(row = {}) {
  const out = invoke('synthesize-envelope', row && typeof row === 'object' ? row : {});
  return out.envelope && typeof out.envelope === 'object' ? out.envelope : null;
}

module.exports = {
  synthesizeEnvelope
};
