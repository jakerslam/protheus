#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const path = require('path');
const { createOpsLaneBridge } = require('../runtime/lib/rust_lane_bridge.ts');

const REPO_ROOT = path.resolve(__dirname, '..');
const DEFAULT_POLICY_PATH = process.env.TRAINING_CONDUIT_POLICY_PATH
  ? path.resolve(String(process.env.TRAINING_CONDUIT_POLICY_PATH))
  : path.join(REPO_ROOT, 'config', 'training_conduit_policy.json');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'training_conduit_schema', 'training-conduit-schema-kernel');

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
      : (out && out.stderr ? String(out.stderr).trim() : `training_conduit_schema_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `training_conduit_schema_kernel_${command}_failed`);
    return { ok: false, error: message || `training_conduit_schema_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `training_conduit_schema_kernel_${command}_bridge_failed`
      : `training_conduit_schema_kernel_${command}_bridge_failed`;
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

function loadTrainingConduitPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const out = invoke('load-policy', {
    root_dir: REPO_ROOT,
    policy_path: policyPath
  });
  return out.policy && typeof out.policy === 'object' ? out.policy : defaultPolicy();
}

function buildTrainingConduitMetadata(input = {}, policyInput = null) {
  const out = invoke('build-metadata', {
    root_dir: REPO_ROOT,
    input: input && typeof input === 'object' ? input : {},
    policy: policyInput && typeof policyInput === 'object' ? policyInput : null
  });
  return out.metadata && typeof out.metadata === 'object' ? out.metadata : null;
}

function validateTrainingConduitMetadata(metadata, policyInput = null) {
  const out = invoke('validate-metadata', {
    root_dir: REPO_ROOT,
    metadata: metadata && typeof metadata === 'object' ? metadata : {},
    policy: policyInput && typeof policyInput === 'object' ? policyInput : null
  });
  return out.validation && typeof out.validation === 'object'
    ? out.validation
    : { ok: false, errors: ['validation_missing'] };
}

module.exports = {
  DEFAULT_POLICY_PATH,
  defaultPolicy,
  normalizePolicy,
  loadTrainingConduitPolicy,
  buildTrainingConduitMetadata,
  validateTrainingConduitMetadata
};
