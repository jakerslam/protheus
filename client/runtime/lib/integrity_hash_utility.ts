'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const path = require('path');
const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');
const { normalizeOpsBridgeEnvAliases } = require('./queued_backlog_runtime.ts');

const ROOT = path.resolve(__dirname, '..', '..', '..');
normalizeOpsBridgeEnvAliases();
process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_USE_PREBUILT =
  process.env.PROTHEUS_OPS_USE_PREBUILT || process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'integrity_hash_utility', 'integrity-hash-utility-kernel');

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
      : (out && out.stderr ? String(out.stderr).trim() : `integrity_hash_utility_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `integrity_hash_utility_kernel_${command}_failed`);
    return { ok: false, error: message || `integrity_hash_utility_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `integrity_hash_utility_kernel_${command}_bridge_failed`
      : `integrity_hash_utility_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function stableStringify(value) {
  const out = invoke('stable-stringify', { value });
  return String(out.value || 'null');
}

function sha256Hex(value) {
  const out = invoke('sha256-hex', { value });
  return String(out.value || '');
}

function hashFileSha256(filePath) {
  const out = invoke('hash-file-sha256', {
    root_dir: ROOT,
    file_path: filePath,
  });
  return String(out.value || '');
}

module.exports = {
  stableStringify,
  sha256Hex,
  hashFileSha256,
};
