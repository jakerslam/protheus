#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
const bridge = createOpsLaneBridge(__dirname, 'success_criteria_verifier', 'success-criteria-kernel');

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function invokeSuccessCriteriaKernel(command, payload) {
  const args = [command];
  if (payload !== undefined) {
    args.push(`--payload-base64=${encodeBase64(JSON.stringify(payload))}`);
  }
  const out = bridge.run(args);
  const payloadOut = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  return {
    ok: !!(payloadOut && payloadOut.ok === true),
    out,
    payload: payloadOut && payloadOut.payload && typeof payloadOut.payload === 'object'
      ? payloadOut.payload
      : payloadOut
  };
}

function parseSuccessCriteriaRows(proposal, opts = {}) {
  const src = opts && typeof opts === 'object' ? opts : {};
  const call = invokeSuccessCriteriaKernel('parse-rows', {
    proposal: proposal && typeof proposal === 'object' ? proposal : {},
    capability_key: src.capability_key || ''
  });
  if (!call.payload || !Array.isArray(call.payload.rows)) {
    throw new Error(
      call.out && call.out.stderr
        ? String(call.out.stderr).trim() || 'success_criteria_kernel_parse_rows_failed'
        : 'success_criteria_kernel_parse_rows_failed'
    );
  }
  return call.payload.rows;
}

function evaluateSuccessCriteria(proposal, context, policy) {
  const call = invokeSuccessCriteriaKernel('evaluate', {
    proposal: proposal && typeof proposal === 'object' ? proposal : {},
    context: context && typeof context === 'object' ? context : {},
    policy: policy && typeof policy === 'object' ? policy : {}
  });
  if (!call.payload || !call.payload.result || typeof call.payload.result !== 'object') {
    throw new Error(
      call.out && call.out.stderr
        ? String(call.out.stderr).trim() || 'success_criteria_kernel_evaluate_failed'
        : 'success_criteria_kernel_evaluate_failed'
    );
  }
  return call.payload.result;
}

module.exports = {
  parseSuccessCriteriaRows,
  evaluateSuccessCriteria
};
