#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const os = require('os');
const path = require('path');
const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');

const REPO_ROOT = path.resolve(__dirname, '..');
const DEFAULT_SECRETS_DIR = process.env.SECRET_BROKER_SECRETS_DIR
  ? path.resolve(process.env.SECRET_BROKER_SECRETS_DIR)
  : path.join(os.homedir(), '.config', 'protheus', 'secrets');
const POLICY_PATH = process.env.SECRET_BROKER_POLICY_PATH
  ? path.resolve(process.env.SECRET_BROKER_POLICY_PATH)
  : path.join(REPO_ROOT, 'config', 'secret_broker_policy.json');
const STATE_PATH = process.env.SECRET_BROKER_STATE_PATH
  ? path.resolve(process.env.SECRET_BROKER_STATE_PATH)
  : path.join(REPO_ROOT, 'local', 'state', 'security', 'secret_broker_state.json');
const AUDIT_PATH = process.env.SECRET_BROKER_AUDIT_PATH
  ? path.resolve(process.env.SECRET_BROKER_AUDIT_PATH)
  : path.join(REPO_ROOT, 'local', 'state', 'security', 'secret_broker_audit.jsonl');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
const bridge = createOpsLaneBridge(__dirname, 'secret_broker', 'secret-broker-kernel');

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function invoke(command, payload = {}) {
  const out = bridge.run([
    command,
    `--payload-base64=${encodeBase64(JSON.stringify(payload || {}))}`
  ]);
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  const payloadOut = receipt && receipt.payload && typeof receipt.payload === 'object'
    ? receipt.payload
    : receipt;
  if (!payloadOut || typeof payloadOut !== 'object') {
    return {
      ok: false,
      error: out && out.stderr ? String(out.stderr).trim() || 'secret_broker_kernel_bridge_failed' : 'secret_broker_kernel_bridge_failed'
    };
  }
  return payloadOut;
}

function normalizeOptions(options = {}) {
  return options && typeof options === 'object' ? { ...options } : {};
}

function loadPolicy(policyPathRaw = null) {
  const out = invoke('load-policy', {
    policy_path: policyPathRaw || POLICY_PATH
  });
  return out.policy && typeof out.policy === 'object'
    ? out.policy
    : {
        version: '1.0',
        path: policyPathRaw || POLICY_PATH,
        audit: { include_backend_details: true },
        secrets: {}
      };
}

function loadSecretById(secretId, opts = {}) {
  return invoke('load-secret', {
    secret_id: String(secretId == null ? '' : secretId),
    ...normalizeOptions(opts)
  });
}

function evaluateSecretRotationHealth(opts = {}) {
  return invoke('rotation-health', normalizeOptions(opts));
}

function secretBrokerStatus(opts = {}) {
  return invoke('status', normalizeOptions(opts));
}

function issueSecretHandle(opts = {}) {
  return invoke('issue-handle', normalizeOptions(opts));
}

function resolveSecretHandle(handle, opts = {}) {
  return invoke('resolve-handle', {
    handle: String(handle == null ? '' : handle),
    ...normalizeOptions(opts)
  });
}

module.exports = {
  issueSecretHandle,
  resolveSecretHandle,
  loadSecretById,
  evaluateSecretRotationHealth,
  secretBrokerStatus,
  loadPolicy,
  POLICY_PATH,
  STATE_PATH,
  AUDIT_PATH,
  DEFAULT_SECRETS_DIR
};
