#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');
const { linkReceiptToPassport } = require('./agent_passport_link.ts');
const { normalizeOpsBridgeEnvAliases } = require('./queued_backlog_runtime.ts');

normalizeOpsBridgeEnvAliases();
process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_USE_PREBUILT =
  process.env.INFRING_OPS_USE_PREBUILT || process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS =
  process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'action_receipts', 'action-receipts-kernel');

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
      : (out && out.stderr ? String(out.stderr).trim() : `action_receipts_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `action_receipts_kernel_${command}_failed`);
    return { ok: false, error: message || `action_receipts_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `action_receipts_kernel_${command}_bridge_failed`
      : `action_receipts_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function nowIso() {
  const out = invoke('now-iso', {});
  return String(out.ts || new Date().toISOString());
}

function appendJsonl(filePath, obj) {
  invoke('append-jsonl', {
    file_path: String(filePath || ''),
    row: obj && typeof obj === 'object' ? obj : obj
  });
}

function withReceiptContract(record, { attempted = true, verified = false } = {}) {
  const out = invoke('with-receipt-contract', {
    record: record && typeof record === 'object' ? record : {},
    attempted,
    verified
  });
  return out.record && typeof out.record === 'object' ? out.record : null;
}

function writeContractReceipt(filePath, record, { attempted = true, verified = false } = {}) {
  const out = invoke('write-contract-receipt', {
    file_path: String(filePath || ''),
    record: record && typeof record === 'object' ? record : {},
    attempted,
    verified
  });
  const written = out.record && typeof out.record === 'object' ? out.record : null;
  if (written) {
    linkReceiptToPassport(String(filePath || ''), written);
  }
  return written;
}

module.exports = {
  nowIso,
  appendJsonl,
  withReceiptContract,
  writeContractReceipt
};
