#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const path = require('path');
const { createOpsLaneBridge } = require('../runtime/lib/rust_lane_bridge.ts');

const REPO_ROOT = path.resolve(__dirname, '..');
const DEFAULT_BURN_ORACLE_LATEST_PATH = process.env.DYNAMIC_BURN_BUDGET_ORACLE_LATEST_PATH
  ? path.resolve(process.env.DYNAMIC_BURN_BUDGET_ORACLE_LATEST_PATH)
  : path.join(REPO_ROOT, 'local', 'state', 'ops', 'dynamic_burn_budget_oracle', 'latest.json');

process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'dynamic_burn_budget_signal', 'dynamic-burn-budget-signal-kernel');

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
      : (out && out.stderr ? String(out.stderr).trim() : `dynamic_burn_budget_signal_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `dynamic_burn_budget_signal_kernel_${command}_failed`);
    return { ok: false, error: message || `dynamic_burn_budget_signal_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `dynamic_burn_budget_signal_kernel_${command}_bridge_failed`
      : `dynamic_burn_budget_signal_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function normalizeBurnPressure(v) {
  const out = invoke('normalize-pressure', { value: v });
  return String(out.pressure || 'none');
}

function pressureRank(v) {
  const out = invoke('pressure-rank', { value: v });
  return Number(out.rank || 0);
}

function mapPressureToCostPressure(v) {
  const out = invoke('cost-pressure', { value: v });
  return Number(out.cost_pressure || 0);
}

function loadDynamicBurnOracleSignal(opts = {}) {
  const payload = opts && typeof opts === 'object' ? { ...opts } : {};
  if (!payload.latest_path && !payload.path) {
    payload.latest_path = DEFAULT_BURN_ORACLE_LATEST_PATH;
  }
  const out = invoke('load-signal', payload);
  return out.signal && typeof out.signal === 'object' ? out.signal : { available: false, pressure: 'none', latest_path: DEFAULT_BURN_ORACLE_LATEST_PATH };
}

module.exports = {
  DEFAULT_BURN_ORACLE_LATEST_PATH,
  normalizeBurnPressure,
  pressureRank,
  mapPressureToCostPressure,
  loadDynamicBurnOracleSignal
};
