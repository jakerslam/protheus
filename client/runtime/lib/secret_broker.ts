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
  : path.join(os.homedir(), '.config', 'infring', 'secrets');
const POLICY_PATH = process.env.SECRET_BROKER_POLICY_PATH
  ? path.resolve(process.env.SECRET_BROKER_POLICY_PATH)
  : path.join(REPO_ROOT, 'config', 'secret_broker_policy.json');
const STATE_PATH = process.env.SECRET_BROKER_STATE_PATH
  ? path.resolve(process.env.SECRET_BROKER_STATE_PATH)
  : path.join(REPO_ROOT, 'local', 'state', 'security', 'secret_broker_state.json');
const AUDIT_PATH = process.env.SECRET_BROKER_AUDIT_PATH
  ? path.resolve(process.env.SECRET_BROKER_AUDIT_PATH)
  : path.join(REPO_ROOT, 'local', 'state', 'security', 'secret_broker_audit.jsonl');

process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
const bridge = createOpsLaneBridge(__dirname, 'secret_broker', 'secret-broker-kernel', {
  preferLocalCore: true
});

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function normalizeOptions(options = {}) {
  return options && typeof options === 'object' ? { ...options } : {};
}

function normalizeText(value) {
  return String(value == null ? '' : value).trim();
}

function normalizedErrorCode(value, fallback) {
  const text = normalizeText(value);
  return text || fallback;
}

function failure(error, extra = {}) {
  return {
    ok: false,
    error: normalizedErrorCode(error, 'secret_broker_kernel_bridge_failed'),
    ...extra
  };
}

function withRuntimePaths(payload = {}) {
  const options = normalizeOptions(payload);
  if (!normalizeText(options.policy_path)) options.policy_path = POLICY_PATH;
  if (!normalizeText(options.state_path)) options.state_path = STATE_PATH;
  if (!normalizeText(options.audit_path)) options.audit_path = AUDIT_PATH;
  if (!normalizeText(options.secrets_dir)) options.secrets_dir = DEFAULT_SECRETS_DIR;
  return options;
}

function invoke(command, payload = {}) {
  const action = normalizeText(command);
  if (!action) {
    return failure('secret_broker_command_required');
  }
  const out = bridge.run([
    action,
    `--payload-base64=${encodeBase64(JSON.stringify(withRuntimePaths(payload)))}`
  ]);

  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  const payloadOut = receipt && receipt.payload && typeof receipt.payload === 'object'
    ? receipt.payload
    : receipt;

  if (!payloadOut || typeof payloadOut !== 'object') {
    const statusCode = Number.isFinite(Number(out && out.status)) ? Number(out.status) : null;
    const stderr = normalizeText(out && out.stderr);
    return failure(stderr || 'secret_broker_kernel_bridge_failed', {
      status: statusCode
    });
  }
  return payloadOut;
}

function normalizeSecretId(secretId) {
  return normalizeText(secretId);
}

function loadPolicy(policyPathRaw = null) {
  const policyPath = normalizeText(policyPathRaw) || POLICY_PATH;
  const out = invoke('load-policy', {
    policy_path: policyPath
  });
  return out.policy && typeof out.policy === 'object'
    ? out.policy
    : {
        version: '1.0',
        path: policyPath,
        audit: { include_backend_details: true },
        secrets: {}
      };
}

function loadSecretById(secretId, opts = {}) {
  const normalizedSecretId = normalizeSecretId(secretId);
  if (!normalizedSecretId) return failure('secret_id_required');
  return invoke('load-secret', {
    secret_id: normalizedSecretId,
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
  const normalizedHandle = normalizeText(handle);
  if (!normalizedHandle) return failure('handle_required');
  return invoke('resolve-handle', {
    handle: normalizedHandle,
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
