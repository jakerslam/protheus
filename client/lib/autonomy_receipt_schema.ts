#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../runtime/lib/rust_lane_bridge.ts');

process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'autonomy_receipt_schema', 'autonomy-receipt-schema-kernel');

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
      : (out && out.stderr ? String(out.stderr).trim() : `autonomy_receipt_schema_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `autonomy_receipt_schema_kernel_${command}_failed`);
    return { ok: false, error: message || `autonomy_receipt_schema_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `autonomy_receipt_schema_kernel_${command}_bridge_failed`
      : `autonomy_receipt_schema_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function toSuccessCriteriaRecord(criteria, fallback = {}) {
  const out = invoke('to-success-criteria-record', {
    criteria,
    fallback: fallback && typeof fallback === 'object' ? fallback : {}
  });
  return out.record && typeof out.record === 'object' ? out.record : null;
}

function withSuccessCriteriaVerification(baseVerification, successCriteria, options = {}) {
  const out = invoke('with-success-criteria-verification', {
    baseVerification: baseVerification && typeof baseVerification === 'object' ? baseVerification : {},
    successCriteria,
    options: options && typeof options === 'object' ? options : {}
  });
  return out.verification && typeof out.verification === 'object' ? out.verification : null;
}

function normalizeAutonomyReceiptForWrite(receipt) {
  const out = invoke('normalize-receipt', {
    receipt: receipt && typeof receipt === 'object' ? receipt : {}
  });
  return out.receipt && typeof out.receipt === 'object' ? out.receipt : null;
}

function successCriteriaFromReceipt(rec) {
  const out = invoke('success-criteria-from-receipt', {
    receipt: rec && typeof rec === 'object' ? rec : {}
  });
  return out.success_criteria && typeof out.success_criteria === 'object'
    ? out.success_criteria
    : null;
}

module.exports = {
  toSuccessCriteriaRecord,
  withSuccessCriteriaVerification,
  normalizeAutonomyReceiptForWrite,
  successCriteriaFromReceipt
};
