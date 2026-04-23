#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const path = require('path');
const { createOpsLaneBridge } = require('../runtime/lib/rust_lane_bridge.ts');

const REPO_ROOT = path.resolve(__dirname, '..');
const DEFAULT_POLICY_PATH = process.env.TRAINABILITY_MATRIX_POLICY_PATH
  ? path.resolve(String(process.env.TRAINABILITY_MATRIX_POLICY_PATH))
  : path.join(REPO_ROOT, 'config', 'trainability_matrix_policy.json');

process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'trainability_matrix', 'trainability-matrix-kernel');

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
      : (out && out.stderr ? String(out.stderr).trim() : `trainability_matrix_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `trainability_matrix_kernel_${command}_failed`);
    return { ok: false, error: message || `trainability_matrix_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `trainability_matrix_kernel_${command}_bridge_failed`
      : `trainability_matrix_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function defaultPolicy() {
  const out = invoke('default-policy', { root_dir: REPO_ROOT });
  return out.policy && typeof out.policy === 'object' ? out.policy : {};
}

function normalizePolicy(raw) {
  const out = invoke('normalize-policy', {
    root_dir: REPO_ROOT,
    policy: raw && typeof raw === 'object' ? raw : {}
  });
  return out.policy && typeof out.policy === 'object' ? out.policy : defaultPolicy();
}

function loadTrainabilityMatrixPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const out = invoke('load-policy', {
    root_dir: REPO_ROOT,
    policy_path: policyPath
  });
  return out.policy && typeof out.policy === 'object' ? out.policy : defaultPolicy();
}

function evaluateTrainingDatumTrainability(metadata, policyInput = null) {
  const out = invoke('evaluate', {
    root_dir: REPO_ROOT,
    metadata: metadata && typeof metadata === 'object' ? metadata : {},
    policy: policyInput && typeof policyInput === 'object' ? policyInput : null
  });
  return out.evaluation && typeof out.evaluation === 'object'
    ? out.evaluation
    : { allow: false, provider: 'unknown', reason: 'evaluation_missing', reasons: ['evaluation_missing'], checks: {} };
}

module.exports = {
  DEFAULT_POLICY_PATH,
  defaultPolicy,
  normalizePolicy,
  loadTrainabilityMatrixPolicy,
  evaluateTrainingDatumTrainability
};
