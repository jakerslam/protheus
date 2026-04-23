#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

const ROOT = path.resolve(__dirname, '../..');

function resetModule(modulePath) {
  delete require.cache[require.resolve(modulePath)];
  return require(modulePath);
}

function main() {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'trit-shadow-rust-'));
  const clientRoot = path.join(tempRoot, 'client');
  const configDir = path.join(clientRoot, 'config');
  const stateDir = path.join(clientRoot, 'local', 'state', 'autonomy');
  const reportsDir = path.join(stateDir, 'trit_shadow_reports');
  const calibrationDir = path.join(stateDir, 'trit_shadow_calibration');
  fs.mkdirSync(configDir, { recursive: true });
  fs.mkdirSync(reportsDir, { recursive: true });
  fs.mkdirSync(calibrationDir, { recursive: true });

  const policyPath = path.join(configDir, 'trit_shadow_policy.json');
  const successCriteriaPath = path.join(configDir, 'trit_shadow_success_criteria.json');
  const trustStatePath = path.join(stateDir, 'trit_shadow_trust_state.json');
  const influenceBudgetPath = path.join(stateDir, 'trit_shadow_influence_budget.json');
  const influenceGuardPath = path.join(stateDir, 'trit_shadow_influence_guard.json');
  const reportHistoryPath = path.join(reportsDir, 'history.jsonl');
  const calibrationHistoryPath = path.join(calibrationDir, 'history.jsonl');
  const originalEnv = {
    AUTONOMY_TRIT_SHADOW_POLICY_PATH: process.env.AUTONOMY_TRIT_SHADOW_POLICY_PATH,
    AUTONOMY_TRIT_SHADOW_SUCCESS_CRITERIA_PATH: process.env.AUTONOMY_TRIT_SHADOW_SUCCESS_CRITERIA_PATH,
    AUTONOMY_TRIT_SHADOW_TRUST_STATE_PATH: process.env.AUTONOMY_TRIT_SHADOW_TRUST_STATE_PATH,
    AUTONOMY_TRIT_SHADOW_INFLUENCE_BUDGET_PATH: process.env.AUTONOMY_TRIT_SHADOW_INFLUENCE_BUDGET_PATH,
    AUTONOMY_TRIT_SHADOW_INFLUENCE_GUARD_PATH: process.env.AUTONOMY_TRIT_SHADOW_INFLUENCE_GUARD_PATH,
    AUTONOMY_TRIT_SHADOW_REPORT_HISTORY_PATH: process.env.AUTONOMY_TRIT_SHADOW_REPORT_HISTORY_PATH,
    AUTONOMY_TRIT_SHADOW_CALIBRATION_HISTORY_PATH: process.env.AUTONOMY_TRIT_SHADOW_CALIBRATION_HISTORY_PATH,
    INFRING_OPS_USE_PREBUILT: process.env.INFRING_OPS_USE_PREBUILT
  };

  fs.writeFileSync(policyPath, JSON.stringify({
    version: '2.0',
    influence: {
      stage: 1,
      max_overrides_per_day: 2,
      auto_disable_hours_on_regression: 24,
      activation: {
        enabled: true,
        report_window: 1,
        min_decisions: 20,
        max_divergence_rate: 0.08,
        require_success_criteria_pass: true,
        require_safety_pass: true,
        require_drift_non_increasing: true,
        calibration_window: 1,
        min_calibration_events: 20,
        min_calibration_accuracy: 0.58,
        max_calibration_ece: 0.23,
        min_source_samples: 8,
        min_source_hit_rate: 0.55,
        max_sources_below_threshold: 1,
        allow_if_no_source_data: false
      },
      auto_stage: {
        enabled: true,
        mode: 'floor',
        stage2: {
          consecutive_reports: 1,
          min_calibration_reports: 1,
          min_decisions: 20,
          max_divergence_rate: 0.08,
          min_calibration_events: 20,
          min_calibration_accuracy: 0.55,
          max_calibration_ece: 0.25,
          require_source_reliability: false
        },
        stage3: {
          consecutive_reports: 2,
          min_calibration_reports: 2,
          min_decisions: 40,
          max_divergence_rate: 0.05,
          min_calibration_events: 40,
          min_calibration_accuracy: 0.65,
          max_calibration_ece: 0.2,
          require_source_reliability: false
        }
      }
    }
  }, null, 2));

  fs.writeFileSync(successCriteriaPath, JSON.stringify({
    version: '1.2',
    targets: { max_divergence_rate: 0.05 }
  }, null, 2));

  fs.writeFileSync(
    reportHistoryPath,
    '{"type":"trit_shadow_report","ok":true,"ts":"2026-03-17T00:00:00Z","summary":{"total_decisions":30,"divergence_rate":0.01},"success_criteria":{"pass":true,"checks":{"safety_regressions":{"pass":true},"drift_non_increasing":{"pass":true}}}}\n'
  );
  fs.writeFileSync(
    calibrationHistoryPath,
    '{"type":"trit_shadow_replay_calibration","ok":true,"ts":"2026-03-17T00:10:00Z","date":"2026-03-17","summary":{"total_events":30,"accuracy":0.70,"expected_calibration_error":0.10},"source_reliability":[{"source":"policy","samples":12,"hit_rate":0.70}]}\n'
  );

  process.env.AUTONOMY_TRIT_SHADOW_POLICY_PATH = policyPath;
  process.env.AUTONOMY_TRIT_SHADOW_SUCCESS_CRITERIA_PATH = successCriteriaPath;
  process.env.AUTONOMY_TRIT_SHADOW_TRUST_STATE_PATH = trustStatePath;
  process.env.AUTONOMY_TRIT_SHADOW_INFLUENCE_BUDGET_PATH = influenceBudgetPath;
  process.env.AUTONOMY_TRIT_SHADOW_INFLUENCE_GUARD_PATH = influenceGuardPath;
  process.env.AUTONOMY_TRIT_SHADOW_REPORT_HISTORY_PATH = reportHistoryPath;
  process.env.AUTONOMY_TRIT_SHADOW_CALIBRATION_HISTORY_PATH = calibrationHistoryPath;
  process.env.INFRING_OPS_USE_PREBUILT = '0';

  const mod = resetModule(path.join(ROOT, 'client/lib/trit_shadow_control.ts'));

  const defaults = mod.defaultTritShadowPolicy();
  assert.strictEqual(defaults.version, '1.0');

  const normalized = mod.normalizeTritShadowPolicy({
    influence: { stage: 8, max_overrides_per_day: -5, auto_stage: { mode: 'override' } }
  });
  assert.strictEqual(normalized.influence.stage, 3);
  assert.strictEqual(normalized.influence.max_overrides_per_day, 0);
  assert.strictEqual(normalized.influence.auto_stage.mode, 'override');

  const loadedPolicy = mod.loadTritShadowPolicy();
  assert.strictEqual(loadedPolicy.version, '2.0');

  const successCriteria = mod.loadTritShadowSuccessCriteria();
  assert.strictEqual(successCriteria.version, '1.2');

  mod.saveTritShadowTrustState({
    default_source_trust: 1.2,
    by_source: {
      policy: { trust: 1.4, samples: 10, hit_rate: 0.7 }
    }
  });
  const trustState = mod.loadTritShadowTrustState(loadedPolicy);
  assert.strictEqual(trustState.by_source.policy.trust, 1.4);
  const trustMap = mod.buildTritSourceTrustMap(trustState);
  assert.strictEqual(trustMap.policy, 1.4);

  const productivity = mod.evaluateTritShadowProductivity(loadedPolicy);
  assert.strictEqual(productivity.active, true);
  const autoStage = mod.evaluateAutoStage(loadedPolicy);
  assert.strictEqual(autoStage.stage, 2);
  const decision = mod.resolveTritShadowStageDecision(loadedPolicy);
  assert.strictEqual(decision.stage, 2);
  assert.strictEqual(mod.resolveTritShadowStage(loadedPolicy), 2);

  const firstCheck = mod.canConsumeTritShadowOverride(loadedPolicy, '2026-03-17');
  assert.strictEqual(firstCheck.allowed, true);
  const firstConsume = mod.consumeTritShadowOverride('planner', loadedPolicy, '2026-03-17');
  const secondConsume = mod.consumeTritShadowOverride('planner', loadedPolicy, '2026-03-17');
  const thirdConsume = mod.consumeTritShadowOverride('planner', loadedPolicy, '2026-03-17');
  assert.strictEqual(firstConsume.consumed, true);
  assert.strictEqual(secondConsume.consumed, true);
  assert.strictEqual(thirdConsume.consumed, false);

  const guard = mod.applyInfluenceGuardFromShadowReport({
    ts: '2026-03-17T12:00:00Z',
    summary: {
      status: 'critical',
      gate: { enabled: true, pass: false, reason: 'divergence_rate_exceeds_limit' }
    }
  }, loadedPolicy);
  assert.strictEqual(guard.disabled, true);
  const blocked = mod.isTritShadowInfluenceBlocked(guard, '2026-03-17T12:30:00Z');
  assert.strictEqual(blocked.blocked, true);

  assert.strictEqual(mod.paths.policy, policyPath);
  assert.strictEqual(mod.paths.report_history, reportHistoryPath);

  assertNoPlaceholderOrPromptLeak(
    { loadedPolicy, trustState, productivity, autoStage, decision, guard },
    'trit_shadow_control_rust_bridge_test'
  );
  assertStableToolingEnvelope(
    { loadedPolicy, trustState, productivity, autoStage, decision, guard },
    'trit_shadow_control_rust_bridge_test'
  );

  for (const [key, value] of Object.entries(originalEnv)) {
    if (value == null) {
      delete process.env[key];
    } else {
      process.env[key] = value;
    }
  }
  fs.rmSync(tempRoot, { recursive: true, force: true });
  console.log(JSON.stringify({ ok: true, type: 'trit_shadow_control_rust_bridge_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
