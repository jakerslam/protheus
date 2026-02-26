#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

function writeFile(filePath, text) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, text, 'utf8');
}

function writeJson(filePath, value) {
  writeFile(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function runNode(scriptPath, args, env, cwd) {
  return spawnSync(process.execPath, [scriptPath, ...args], {
    cwd,
    env,
    encoding: 'utf8'
  });
}

function parseJsonStdout(proc) {
  const raw = String(proc.stdout || '').trim();
  assert.ok(raw, 'expected JSON stdout');
  return JSON.parse(raw);
}

function assertOk(proc, label) {
  assert.strictEqual(proc.status, 0, `${label} failed: ${proc.stderr || proc.stdout}`);
  const out = parseJsonStdout(proc);
  assert.strictEqual(out.ok, true, `${label} expected ok=true`);
  return out;
}

function basePolicy() {
  return {
    version: '1.0-test',
    enabled: true,
    shadow_only: true,
    allow_apply: false,
    max_candidates_per_run: 6,
    trigger: {
      min_uses: 2,
      min_workflow_spread: 2,
      min_days_observed: 0,
      min_pain_score: 0,
      cooldown_after_failure_hours: 0,
      cooldown_after_rejection_hours: 0
    },
    legal_gate: {
      fail_closed: true,
      require_license_check: true,
      require_tos_check: true,
      require_robots_check: true,
      require_data_rights: true,
      denied_licenses: ['gpl-3.0'],
      allowed_licenses: [],
      blocked_domains: []
    },
    anti_gaming: {
      hidden_eval_min_cases: 3,
      hidden_eval_max_cases: 7,
      retry_rate_limit_per_capability_per_day: 10
    },
    risk_classes: {
      high_risk: ['payments', 'auth', 'filesystem', 'shell', 'network-control'],
      require_explicit_human_approval: true
    },
    assimilation_scope: {
      max_assimilation_depth: 2,
      approval_threshold_score: 0.7,
      resource_budget_gate: {
        enabled: false
      },
      atrophy: {
        enabled: true,
        dormant_after_days: 30,
        compression: 'zstd'
      }
    },
    research_probe: {
      min_confidence: 0.4
    },
    integration: {
      weaver_latest_path: 'state/autonomy/weaver/latest.json',
      nursery_shadow_only: true,
      adversarial_shadow_only: true
    },
    outputs: {
      emit_events: true,
      emit_ide_events: true,
      emit_obsidian_projection: true
    }
  };
}

function run() {
  const repoRoot = path.resolve(__dirname, '..', '..', '..');
  const scriptPath = path.join(repoRoot, 'systems', 'assimilation', 'assimilation_controller.js');
  const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'assimilation-controller-'));
  const policyPath = path.join(tmpRoot, 'config', 'assimilation_policy.json');
  const stateDir = path.join(tmpRoot, 'state', 'assimilation');
  const weaverLatestPath = path.join(tmpRoot, 'state', 'autonomy', 'weaver', 'latest.json');

  writeJson(weaverLatestPath, {
    ts: new Date().toISOString(),
    veto_blocked: false,
    value_context: {
      constitutional_veto: {
        blocked: false
      },
      primary_metric_id: 'adaptive_value',
      value_currency: 'adaptive_value'
    }
  });
  writeJson(policyPath, basePolicy());

  const env = {
    ...process.env,
    ASSIMILATION_POLICY_PATH: policyPath,
    ASSIMILATION_STATE_DIR: stateDir,
    ASSIMILATION_WEAVER_LATEST_PATH: weaverLatestPath
  };

  // Unified candidacy ledger must accept both local skills and external adapters.
  assertOk(runNode(scriptPath, [
    'record-use',
    '--capability-id=cap.local.alpha',
    '--source-type=local_skill',
    '--workflow-id=wf_local_a',
    '--success=1',
    '--pain-score=0.2',
    '--license=mit',
    '--tos-ok=1',
    '--robots-ok=1',
    '--data-rights-ok=1'
  ], env, repoRoot), 'record local alpha #1');
  assertOk(runNode(scriptPath, [
    'record-use',
    '--capability-id=cap.local.alpha',
    '--source-type=local_skill',
    '--workflow-id=wf_local_b',
    '--success=1',
    '--pain-score=0.2',
    '--license=mit',
    '--tos-ok=1',
    '--robots-ok=1',
    '--data-rights-ok=1'
  ], env, repoRoot), 'record local alpha #2');

  assertOk(runNode(scriptPath, [
    'record-use',
    '--capability-id=cap.external.beta',
    '--source-type=external_adapter',
    '--workflow-id=wf_ext_a',
    '--success=1',
    '--pain-score=0.2',
    '--license=mit',
    '--tos-ok=1',
    '--robots-ok=1',
    '--data-rights-ok=1'
  ], env, repoRoot), 'record external beta #1');
  assertOk(runNode(scriptPath, [
    'record-use',
    '--capability-id=cap.external.beta',
    '--source-type=external_adapter',
    '--workflow-id=wf_ext_b',
    '--success=1',
    '--pain-score=0.2',
    '--license=mit',
    '--tos-ok=1',
    '--robots-ok=1',
    '--data-rights-ok=1'
  ], env, repoRoot), 'record external beta #2');

  const assess = assertOk(runNode(scriptPath, ['assess'], env, repoRoot), 'assess');
  const readyIds = new Set((assess.candidates || []).map((row) => String(row.capability_id || '')));
  assert.ok(readyIds.has('cap.local.alpha'), 'local skill candidate should be ready');
  assert.ok(readyIds.has('cap.external.beta'), 'external adapter candidate should be ready');

  const runShadow = assertOk(runNode(scriptPath, [
    'run',
    '2026-02-26',
    '--apply=1'
  ], env, repoRoot), 'run shadow');
  assert.ok((runShadow.candidates || []).length >= 2, 'shadow run should process ready candidates');
  for (const row of runShadow.candidates || []) {
    assert.strictEqual(row.outcome, 'shadow_only', 'shadow mode should not execute live graft');
    assert.strictEqual(row.graft.apply_executed, false, 'shadow mode should keep apply_executed=false');
  }

  const ledgerPath = path.join(stateDir, 'ledger.json');
  const ledger = JSON.parse(fs.readFileSync(ledgerPath, 'utf8'));
  assert.strictEqual(ledger.capabilities['cap.local.alpha'].source_type, 'local_skill');
  assert.strictEqual(ledger.capabilities['cap.external.beta'].source_type, 'external_adapter');

  // Move to live-eligible mode and validate high-risk approval gating.
  const livePolicy = basePolicy();
  livePolicy.shadow_only = false;
  livePolicy.allow_apply = true;
  writeJson(policyPath, livePolicy);

  assertOk(runNode(scriptPath, [
    'record-use',
    '--capability-id=cap.external.shell',
    '--source-type=external_tool',
    '--risk-class=shell',
    '--workflow-id=wf_shell_a',
    '--success=1',
    '--pain-score=0.3',
    '--license=mit',
    '--tos-ok=1',
    '--robots-ok=1',
    '--data-rights-ok=1'
  ], env, repoRoot), 'record shell #1');
  assertOk(runNode(scriptPath, [
    'record-use',
    '--capability-id=cap.external.shell',
    '--source-type=external_tool',
    '--risk-class=shell',
    '--workflow-id=wf_shell_b',
    '--success=1',
    '--pain-score=0.3',
    '--license=mit',
    '--tos-ok=1',
    '--robots-ok=1',
    '--data-rights-ok=1'
  ], env, repoRoot), 'record shell #2');

  const runRejected = assertOk(runNode(scriptPath, [
    'run',
    '2026-02-26',
    '--capability-id=cap.external.shell',
    '--apply=1'
  ], env, repoRoot), 'run high risk unapproved');
  assert.strictEqual(runRejected.candidates.length, 1);
  assert.strictEqual(runRejected.candidates[0].outcome, 'reject');
  assert.ok(
    (runRejected.candidates[0].graft.reason_codes || []).includes('graft_blocked_high_risk_requires_human_approval'),
    'high-risk capability should require human approval'
  );

  const runApproved = assertOk(runNode(scriptPath, [
    'run',
    '2026-02-26',
    '--capability-id=cap.external.shell',
    '--apply=1',
    '--human-approved=1'
  ], env, repoRoot), 'run high risk approved');
  assert.strictEqual(runApproved.candidates.length, 1);
  assert.strictEqual(runApproved.candidates[0].outcome, 'success');
  assert.strictEqual(runApproved.candidates[0].graft.apply_executed, true);

  const status = assertOk(runNode(scriptPath, ['status', 'latest'], env, repoRoot), 'status latest');
  assert.ok(Number(status.candidates_processed || 0) >= 1, 'status should include processed count');
  assert.strictEqual(status.shadow_only, false, 'latest status should reflect live policy');

  console.log('assimilation_controller.test.js: OK');
}

try {
  run();
} catch (err) {
  console.error(`assimilation_controller.test.js: FAIL: ${err.message}`);
  process.exit(1);
}
