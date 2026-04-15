#!/usr/bin/env node
'use strict';

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
const bridge = createOpsLaneBridge(__dirname, 'ternary_belief_engine', 'ternary-belief-kernel');

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
    throw new Error(out && out.stderr ? String(out.stderr).trim() || 'ternary_belief_kernel_bridge_failed' : 'ternary_belief_kernel_bridge_failed');
  }
  return payloadOut;
}

function evaluateTernaryBelief(signals = [], opts = {}) {
  return invoke('evaluate', {
    signals: Array.isArray(signals) ? signals : [],
    opts: opts && typeof opts === 'object' ? { ...opts } : {}
  });
}

function mergeTernaryBeliefs(parentBelief = {}, childBelief = {}, opts = {}) {
  return invoke('merge', {
    parent_belief: parentBelief && typeof parentBelief === 'object' ? { ...parentBelief } : {},
    child_belief: childBelief && typeof childBelief === 'object' ? { ...childBelief } : {},
    opts: opts && typeof opts === 'object' ? { ...opts } : {}
  });
}

function serializeBeliefResult(result = {}) {
  return invoke('serialize', {
    result: result && typeof result === 'object' ? { ...result } : {}
  });
}

module.exports = {
  evaluateTernaryBelief,
  mergeTernaryBeliefs,
  serializeBeliefResult
};
