#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');
const { normalizeOpsBridgeEnvAliases } = require('./queued_backlog_runtime.ts');

normalizeOpsBridgeEnvAliases();
process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_USE_PREBUILT =
  process.env.PROTHEUS_OPS_USE_PREBUILT || process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'quorum_validator', 'quorum-validator-kernel');

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
      : (out && out.stderr ? String(out.stderr).trim() : `quorum_validator_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `quorum_validator_kernel_${command}_failed`);
    return { ok: false, error: message || `quorum_validator_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `quorum_validator_kernel_${command}_bridge_failed`
      : `quorum_validator_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function evaluateProposalQuorum(proposal) {
  const out = invoke('evaluate', {
    proposal: proposal && typeof proposal === 'object' ? proposal : {}
  });
  return out.verdict && typeof out.verdict === 'object'
    ? out.verdict
    : {
        requires_quorum: false,
        ok: false,
        agreement: false,
        reason: 'quorum_validator_kernel_bridge_failed',
        passes: []
      };
}

module.exports = {
  evaluateProposalQuorum
};
