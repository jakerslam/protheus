#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const path = require('path');
const { createOpsLaneBridge } = require('../runtime/lib/rust_lane_bridge.ts');

function repoRoot() {
  return path.resolve(__dirname, '..');
}

const POLICY_PATH = process.env.AUTONOMY_TRIT_SHADOW_POLICY_PATH
  ? path.resolve(process.env.AUTONOMY_TRIT_SHADOW_POLICY_PATH)
  : path.join(repoRoot(), 'config', 'trit_shadow_policy.json');
const SUCCESS_CRITERIA_PATH = process.env.AUTONOMY_TRIT_SHADOW_SUCCESS_CRITERIA_PATH
  ? path.resolve(process.env.AUTONOMY_TRIT_SHADOW_SUCCESS_CRITERIA_PATH)
  : path.join(repoRoot(), 'config', 'trit_shadow_success_criteria.json');
const TRUST_STATE_PATH = process.env.AUTONOMY_TRIT_SHADOW_TRUST_STATE_PATH
  ? path.resolve(process.env.AUTONOMY_TRIT_SHADOW_TRUST_STATE_PATH)
  : path.join(repoRoot(), 'local', 'state', 'autonomy', 'trit_shadow_trust_state.json');
const INFLUENCE_BUDGET_PATH = process.env.AUTONOMY_TRIT_SHADOW_INFLUENCE_BUDGET_PATH
  ? path.resolve(process.env.AUTONOMY_TRIT_SHADOW_INFLUENCE_BUDGET_PATH)
  : path.join(repoRoot(), 'local', 'state', 'autonomy', 'trit_shadow_influence_budget.json');
const INFLUENCE_GUARD_PATH = process.env.AUTONOMY_TRIT_SHADOW_INFLUENCE_GUARD_PATH
  ? path.resolve(process.env.AUTONOMY_TRIT_SHADOW_INFLUENCE_GUARD_PATH)
  : path.join(repoRoot(), 'local', 'state', 'autonomy', 'trit_shadow_influence_guard.json');
const REPORT_HISTORY_PATH = process.env.AUTONOMY_TRIT_SHADOW_REPORT_HISTORY_PATH
  ? path.resolve(process.env.AUTONOMY_TRIT_SHADOW_REPORT_HISTORY_PATH)
  : path.join(repoRoot(), 'local', 'state', 'autonomy', 'trit_shadow_reports', 'history.jsonl');
const CALIBRATION_HISTORY_PATH = process.env.AUTONOMY_TRIT_SHADOW_CALIBRATION_HISTORY_PATH
  ? path.resolve(process.env.AUTONOMY_TRIT_SHADOW_CALIBRATION_HISTORY_PATH)
  : path.join(repoRoot(), 'local', 'state', 'autonomy', 'trit_shadow_calibration', 'history.jsonl');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
const bridge = createOpsLaneBridge(__dirname, 'trit_shadow_control', 'trit-shadow-kernel');

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function normalizeObject(value) {
  return value && typeof value === 'object' ? { ...value } : {};
}

function invoke(command, payload = {}) {
  const out = bridge.run([
    command,
    `--payload-base64=${encodeBase64(JSON.stringify(normalizeObject(payload)))}`
  ]);
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  const payloadOut = receipt && receipt.payload && typeof receipt.payload === 'object'
    ? receipt.payload
    : receipt;
  if (!payloadOut || typeof payloadOut !== 'object') {
    throw new Error(
      out && out.stderr
        ? String(out.stderr).trim() || 'trit_shadow_kernel_bridge_failed'
        : 'trit_shadow_kernel_bridge_failed'
    );
  }
  return payloadOut;
}

function buildPaths(overrides = {}) {
  return {
    policy: overrides.policy || POLICY_PATH,
    success_criteria: overrides.success_criteria || SUCCESS_CRITERIA_PATH,
    report_history: overrides.report_history || REPORT_HISTORY_PATH,
    calibration_history: overrides.calibration_history || CALIBRATION_HISTORY_PATH,
    trust_state: overrides.trust_state || TRUST_STATE_PATH,
    influence_budget: overrides.influence_budget || INFLUENCE_BUDGET_PATH,
    influence_guard: overrides.influence_guard || INFLUENCE_GUARD_PATH
  };
}

function todayStr() {
  return new Date().toISOString().slice(0, 10);
}

function defaultTritShadowPolicy() {
  return invoke('default-policy');
}

function normalizeTritShadowPolicy(input) {
  return invoke('normalize-policy', { policy: normalizeObject(input) });
}

function loadTritShadowPolicy(filePath = POLICY_PATH) {
  return invoke('load-policy', {
    paths: { policy: filePath }
  });
}

function loadTritShadowSuccessCriteria(filePath = SUCCESS_CRITERIA_PATH) {
  return invoke('load-success-criteria', {
    paths: { success_criteria: filePath }
  });
}

function loadTritShadowTrustState(policy = loadTritShadowPolicy(), filePath = TRUST_STATE_PATH) {
  return invoke('load-trust-state', {
    policy: normalizeObject(policy),
    paths: { trust_state: filePath }
  });
}

function saveTritShadowTrustState(state, filePath = TRUST_STATE_PATH) {
  invoke('save-trust-state', {
    state: normalizeObject(state),
    paths: { trust_state: filePath }
  });
}

function buildTritSourceTrustMap(trustState) {
  return invoke('build-trust-map', {
    trust_state: normalizeObject(trustState)
  });
}

function evaluateTritShadowProductivity(policy = loadTritShadowPolicy()) {
  return invoke('evaluate-productivity', {
    policy: normalizeObject(policy),
    paths: buildPaths()
  });
}

function evaluateAutoStage(policy = loadTritShadowPolicy()) {
  return invoke('evaluate-auto-stage', {
    policy: normalizeObject(policy),
    paths: buildPaths()
  });
}

function resolveTritShadowStageDecision(policy) {
  return invoke('resolve-stage-decision', {
    policy: normalizeObject(policy),
    paths: buildPaths()
  });
}

function resolveTritShadowStage(policy) {
  const out = invoke('resolve-stage', {
    policy: normalizeObject(policy),
    paths: buildPaths()
  });
  return Number(out && out.stage || 0);
}

function canConsumeTritShadowOverride(policy, dateStr = todayStr(), filePath = INFLUENCE_BUDGET_PATH) {
  return invoke('can-consume-override', {
    policy: normalizeObject(policy),
    date_str: String(dateStr || todayStr()),
    paths: { influence_budget: filePath }
  });
}

function consumeTritShadowOverride(source, policy, dateStr = todayStr(), filePath = INFLUENCE_BUDGET_PATH) {
  return invoke('consume-override', {
    source: String(source || 'unknown'),
    policy: normalizeObject(policy),
    date_str: String(dateStr || todayStr()),
    paths: { influence_budget: filePath }
  });
}

function loadTritShadowInfluenceGuard(filePath = INFLUENCE_GUARD_PATH) {
  return invoke('load-influence-guard', {
    paths: { influence_guard: filePath }
  });
}

function saveTritShadowInfluenceGuard(guard, filePath = INFLUENCE_GUARD_PATH) {
  invoke('save-influence-guard', {
    guard: normalizeObject(guard),
    paths: { influence_guard: filePath }
  });
}

function isTritShadowInfluenceBlocked(guard, nowTs) {
  const payload = {
    guard: normalizeObject(guard)
  };
  if (nowTs) payload.now_ts = String(nowTs);
  return invoke('influence-blocked', payload);
}

function applyInfluenceGuardFromShadowReport(
  reportPayload,
  policy = loadTritShadowPolicy(),
  filePath = INFLUENCE_GUARD_PATH
) {
  return invoke('apply-influence-guard', {
    report_payload: normalizeObject(reportPayload),
    policy: normalizeObject(policy),
    paths: { influence_guard: filePath }
  });
}

module.exports = {
  defaultTritShadowPolicy,
  normalizeTritShadowPolicy,
  loadTritShadowPolicy,
  loadTritShadowSuccessCriteria,
  loadTritShadowTrustState,
  saveTritShadowTrustState,
  buildTritSourceTrustMap,
  evaluateTritShadowProductivity,
  evaluateAutoStage,
  resolveTritShadowStageDecision,
  resolveTritShadowStage,
  canConsumeTritShadowOverride,
  consumeTritShadowOverride,
  loadTritShadowInfluenceGuard,
  saveTritShadowInfluenceGuard,
  isTritShadowInfluenceBlocked,
  applyInfluenceGuardFromShadowReport,
  paths: buildPaths()
};
