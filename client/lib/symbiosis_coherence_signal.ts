#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const path = require('path');
const { createOpsLaneBridge } = require('../runtime/lib/rust_lane_bridge.ts');

const CLIENT_ROOT = path.resolve(__dirname, '..');
const DEFAULT_POLICY_PATH = process.env.SYMBIOSIS_COHERENCE_POLICY_PATH
  ? path.resolve(process.env.SYMBIOSIS_COHERENCE_POLICY_PATH)
  : path.join(CLIENT_ROOT, 'runtime', 'config', 'symbiosis_coherence_policy.json');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
const bridge = createOpsLaneBridge(__dirname, 'symbiosis_coherence_signal', 'symbiosis-coherence-kernel');

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
      available: false,
      type: 'symbiosis_coherence_signal',
      error: out && out.stderr ? String(out.stderr).trim() || 'symbiosis_coherence_kernel_bridge_failed' : 'symbiosis_coherence_kernel_bridge_failed'
    };
  }
  return payloadOut;
}

function normalizeOptions(options = {}) {
  return options && typeof options === 'object' ? { ...options } : {};
}

function loadSymbiosisCoherencePolicy(policyPath = DEFAULT_POLICY_PATH) {
  const out = invoke('load-policy', {
    policy_path: policyPath || DEFAULT_POLICY_PATH
  });
  return out.policy && typeof out.policy === 'object'
    ? out.policy
    : {
        version: '1.0',
        enabled: true,
        shadow_only: true,
        policy_path: policyPath || DEFAULT_POLICY_PATH,
        paths: {}
      };
}

function evaluateSymbiosisCoherenceSignal(opts = {}) {
  return invoke('evaluate', normalizeOptions(opts));
}

function loadSymbiosisCoherenceSignal(opts = {}) {
  return invoke('load', normalizeOptions(opts));
}

function evaluateRecursionRequest(opts = {}) {
  return invoke('recursion-request', normalizeOptions(opts));
}

module.exports = {
  DEFAULT_POLICY_PATH,
  loadSymbiosisCoherencePolicy,
  evaluateSymbiosisCoherenceSignal,
  loadSymbiosisCoherenceSignal,
  evaluateRecursionRequest
};
