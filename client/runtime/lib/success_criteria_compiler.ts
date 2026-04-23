#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');
const { normalizeOpsBridgeEnvAliases } = require('./queued_backlog_runtime.ts');

normalizeOpsBridgeEnvAliases();
process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_USE_PREBUILT =
  process.env.INFRING_OPS_USE_PREBUILT || process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS =
  process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'success_criteria_compiler', 'success-criteria-compiler-kernel');

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
      : (out && out.stderr ? String(out.stderr).trim() : `success_criteria_compiler_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `success_criteria_compiler_kernel_${command}_failed`);
    return { ok: false, error: message || `success_criteria_compiler_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `success_criteria_compiler_kernel_${command}_bridge_failed`
      : `success_criteria_compiler_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function compileSuccessCriteriaRows(rows, opts = {}) {
  const out = invoke('compile-rows', {
    rows: Array.isArray(rows) ? rows : [],
    source: opts && typeof opts === 'object' ? opts.source : undefined
  });
  return Array.isArray(out.rows) ? out.rows : [];
}

function compileProposalSuccessCriteria(proposal, opts = {}) {
  const out = invoke('compile-proposal', {
    proposal: proposal && typeof proposal === 'object' ? proposal : {},
    opts: opts && typeof opts === 'object' ? opts : {}
  });
  return Array.isArray(out.rows) ? out.rows : [];
}

function toActionSpecRows(compiledRows) {
  const out = invoke('to-action-spec-rows', {
    rows: Array.isArray(compiledRows) ? compiledRows : []
  });
  return Array.isArray(out.rows) ? out.rows : [];
}

module.exports = {
  compileSuccessCriteriaRows,
  compileProposalSuccessCriteria,
  toActionSpecRows
};
