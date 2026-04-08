'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const path = require('path');
const { createOpsLaneBridge } = require('../runtime/lib/rust_lane_bridge.ts');

const ROOT = path.resolve(__dirname, '..', '..');
const DEFAULT_POLICY_PATH = path.join(ROOT, 'client', 'runtime', 'config', 'redaction_classification_policy.json');
process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'redaction_classification', 'redaction-classification-kernel');

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
      : (out && out.stderr ? String(out.stderr).trim() : `redaction_classification_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `redaction_classification_kernel_${command}_failed`);
    return { ok: false, error: message || `redaction_classification_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `redaction_classification_kernel_${command}_bridge_failed`
      : `redaction_classification_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function normalizePolicyPath(policyPath = null) {
  return String(policyPath || DEFAULT_POLICY_PATH);
}

function loadPolicy(policyPath = null) {
  const out = invoke('load-policy', {
    root_dir: ROOT,
    policy_path: normalizePolicyPath(policyPath),
  });
  return out.policy && typeof out.policy === 'object'
    ? out.policy
    : { patterns: [], labels: [], rules: [] };
}

function classifyText(text, policyPath = null) {
  const out = invoke('classify-text', {
    root_dir: ROOT,
    text: String(text == null ? '' : text),
    policy_path: normalizePolicyPath(policyPath),
  });
  return out.classification && typeof out.classification === 'object'
    ? out.classification
    : { ok: true, findings: [], labels: [] };
}

function redactText(text, policyPath = null, replacement = '[REDACTED]') {
  const out = invoke('redact-text', {
    root_dir: ROOT,
    text: String(text == null ? '' : text),
    policy_path: normalizePolicyPath(policyPath),
    replacement: String(replacement),
  });
  return out.redaction && typeof out.redaction === 'object'
    ? out.redaction
    : { ok: true, text: String(text == null ? '' : text), replacement: String(replacement) };
}

function classifyAndRedact(text, policyPath = null, replacement = '[REDACTED]') {
  const out = invoke('classify-and-redact', {
    root_dir: ROOT,
    text: String(text == null ? '' : text),
    policy_path: normalizePolicyPath(policyPath),
    replacement: String(replacement),
  });
  return {
    ok: true,
    classification: out.classification && typeof out.classification === 'object'
      ? out.classification
      : { ok: true, findings: [], labels: [] },
    redaction: out.redaction && typeof out.redaction === 'object'
      ? out.redaction
      : { ok: true, text: String(text == null ? '' : text), replacement: String(replacement) },
  };
}

module.exports = { loadPolicy, classifyText, redactText, classifyAndRedact };
