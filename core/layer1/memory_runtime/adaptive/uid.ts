// Layer ownership: core/layer0/ops (authoritative)
'use strict';
export {};

const { createOpsLaneBridge } = require('../../../../client/runtime/lib/rust_lane_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'uid', 'uid-kernel');
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
      : (out && out.stderr ? String(out.stderr).trim() : `uid_kernel_${command}_failed`);
    if (opts.throwOnError === false) return { ok: false, error: message || `uid_kernel_${command}_failed` };
    throw new Error(message || `uid_kernel_${command}_failed`);
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `uid_kernel_${command}_bridge_failed`
      : `uid_kernel_${command}_bridge_failed`;
    if (opts.throwOnError === false) return { ok: false, error: message };
    throw new Error(message);
  }
  return payloadOut;
}

function isAlnum(v) {
  return invoke('is-alnum', { value: v }).result === true;
}

function stableUid(seed, opts = {}) {
  const out = invoke('stable-uid', {
    seed: String(seed == null ? '' : seed),
    prefix: opts.prefix || '',
    length: opts.length
  });
  return String(out.uid || '');
}

function randomUid(opts = {}) {
  const out = invoke('random-uid', {
    prefix: opts.prefix || '',
    length: opts.length
  });
  return String(out.uid || '');
}

module.exports = {
  isAlnum,
  stableUid,
  randomUid,
  forbiddenRuntimeContextMarkers: FORBIDDEN_RUNTIME_CONTEXT_MARKERS,
  containsForbiddenRuntimeContextMarker
};
