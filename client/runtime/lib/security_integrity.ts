#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const path = require('path');
const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');

function runtimeRoot(rootOverride = null) {
  if (rootOverride) return path.resolve(String(rootOverride));
  if (process.env.PROTHEUS_RUNTIME_ROOT) return path.resolve(process.env.PROTHEUS_RUNTIME_ROOT);
  if (process.env.PROTHEUS_WORKSPACE_ROOT) {
    return path.join(path.resolve(process.env.PROTHEUS_WORKSPACE_ROOT), 'client', 'runtime');
  }
  return path.resolve(__dirname, '..');
}

function defaultPolicyPath(rootOverride = null) {
  return path.join(runtimeRoot(rootOverride), 'config', 'security_integrity_policy.json');
}

function defaultLogPath(rootOverride = null) {
  return path.join(runtimeRoot(rootOverride), 'local', 'state', 'security', 'integrity_violations.jsonl');
}

const DEFAULT_POLICY_PATH = defaultPolicyPath();
const DEFAULT_LOG_PATH = defaultLogPath();

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'security_integrity', 'security-integrity-kernel');

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
      : (out && out.stderr ? String(out.stderr).trim() : `security_integrity_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `security_integrity_kernel_${command}_failed`);
    return { ok: false, error: message || `security_integrity_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `security_integrity_kernel_${command}_bridge_failed`
      : `security_integrity_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function normalizePolicyPath(policyPath, rootOverride = null) {
  if (policyPath) return path.resolve(String(policyPath));
  return defaultPolicyPath(rootOverride);
}

function normalizeLogPath(logPath, rootOverride = null) {
  if (logPath) return path.resolve(String(logPath));
  return defaultLogPath(rootOverride);
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const out = invoke('load-policy', {
    policy_path: normalizePolicyPath(policyPath),
    log_path: normalizeLogPath(null)
  });
  return out.policy || {
    version: '1.0',
    target_roots: ['systems/security', 'config/directives'],
    target_extensions: ['.js', '.yaml', '.yml'],
    protected_files: ['lib/directive_resolver.js'],
    exclude_paths: [],
    hashes: {}
  };
}

function collectPresentProtectedFiles(policyOrPath = null) {
  const payload = {};
  if (typeof policyOrPath === 'string') {
    payload.policy_path = normalizePolicyPath(policyOrPath);
  } else if (policyOrPath && typeof policyOrPath === 'object' && !Array.isArray(policyOrPath)) {
    payload.policy = policyOrPath;
    payload.policy_path = normalizePolicyPath(null);
  } else {
    payload.policy_path = normalizePolicyPath(null);
  }
  const out = invoke('collect-present-files', payload);
  return Array.isArray(out.files) ? out.files : [];
}

function verifyIntegrity(policyPath = DEFAULT_POLICY_PATH) {
  return invoke('verify', {
    policy_path: normalizePolicyPath(policyPath),
    log_path: normalizeLogPath(null)
  });
}

function sealIntegrity(policyPath = DEFAULT_POLICY_PATH, options = {}) {
  return invoke('seal', {
    policy_path: normalizePolicyPath(policyPath),
    approval_note: options && typeof options === 'object' ? options.approval_note : undefined,
    sealed_by: options && typeof options === 'object' ? options.sealed_by : undefined
  });
}

function appendIntegrityEvent(entry, logPath = DEFAULT_LOG_PATH) {
  return invoke('append-event', {
    entry: entry && typeof entry === 'object' ? entry : {},
    log_path: normalizeLogPath(logPath)
  }, { throwOnError: false });
}

module.exports = {
  DEFAULT_POLICY_PATH,
  DEFAULT_LOG_PATH,
  loadPolicy,
  collectPresentProtectedFiles,
  verifyIntegrity,
  sealIntegrity,
  appendIntegrityEvent
};
