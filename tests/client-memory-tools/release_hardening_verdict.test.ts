#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

require('../../client/runtime/lib/ts_bootstrap.ts').installTsRequireHook();

const { buildReport: buildScorecardReport } = require('../../tests/tooling/scripts/ci/release_scorecard_generate.ts');
const { buildReport: buildVerdictReport } = require('../../tests/tooling/scripts/ci/release_verdict_gate.ts');

function writeJson(filePath, payload) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function testScorecardTrendRegression() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'release-scorecard-test-'));
  writeJson(path.join(root, 'semver.json'), {
    ok: true,
    next_tag: 'v0.3.11-alpha',
    next_version: '0.3.11-alpha',
    release_channel: 'alpha',
  });
  writeJson(path.join(root, 'commit.json'), { ok: true, strict: true, invalid_count: 0 });
  writeJson(path.join(root, 'policy.json'), { ok: true });
  writeJson(path.join(root, 'canary.json'), { ok: true });
  writeJson(path.join(root, 'closure_policy.json'), {
    numeric_thresholds: {
      ipc_success_rate_min: 0.95,
      receipt_completeness_rate_min: 1,
      supported_command_latency_ms_max: 2500,
      recovery_rto_minutes_max: 30,
      recovery_rpo_hours_max: 24,
    },
  });
  writeJson(path.join(root, 'support_bundle.json'), {
    incident_truth_package: { ready: true, failed_checks: [] },
    metrics: {
      receipt_completeness_rate: 1,
      supported_command_latency_ms: 900,
      max_command_latency_ms: 900,
    },
  });
  writeJson(path.join(root, 'topology.json'), { ok: true, supported_production_topology: true, support_level: 'production_supported' });
  writeJson(path.join(root, 'state.json'), {
    ok: true,
    checks: {
      live_taskgroup_rehearsal_verified: true,
      live_receipt_rehearsal_verified: true,
      live_memory_surface_verified: true,
      live_runtime_receipt_verified: true,
      live_assimilation_contract_verified: true,
    },
  });
  writeJson(path.join(root, 'blockers.json'), { ok: true, open_release_blockers: [], release_blocker_budget_remaining: 0 });
  writeJson(path.join(root, 'closure.json'), { ok: true, summary: { pass: true }, failed_ids: [] });
  writeJson(path.join(root, 'hardening.json'), { ok: true, active: false, violations: [] });
  writeJson(path.join(root, 'ipc.json'), { rows: [{ ok: true }, { ok: true }, { ok: true }] });
  writeJson(path.join(root, 'dr.json'), { observed_rto_minutes: 8, observed_rpo_hours: 2 });
  writeJson(path.join(root, 'baseline.json'), {
    tag: 'v0.3.10-alpha',
    version: '0.3.10-alpha',
    thresholds: {
      ipc_success_rate: 1,
      receipt_completeness_rate: 1,
      max_command_latency_ms: 700,
      observed_rto_minutes: 8,
      observed_rpo_hours: 2,
    },
  });
  fs.mkdirSync(path.join(root, 'client/runtime/local/state/release'), { recursive: true });
  fs.writeFileSync(path.join(root, 'client/runtime/local/state/release/CHANGELOG.auto.md'), 'changelog\n', 'utf8');

  const result = buildScorecardReport({
    strict: true,
    rootPath: root,
    outPath: 'out.json',
    semverPath: 'semver.json',
    commitLintPath: 'commit.json',
    policyPath: 'policy.json',
    canaryPath: 'canary.json',
    changelogPath: 'client/runtime/local/state/release/CHANGELOG.auto.md',
    closurePolicyPath: 'closure_policy.json',
    supportBundlePath: 'support_bundle.json',
    topologyPath: 'topology.json',
    stateCompatPath: 'state.json',
    blockersPath: 'blockers.json',
    closurePath: 'closure.json',
    hardeningPath: 'hardening.json',
    ipcSoakPath: 'ipc.json',
    drPath: 'dr.json',
    baselinePath: 'baseline.json',
    baselineTag: 'v0.3.10-alpha',
    requireBaseline: true,
    requireReleaseArtifacts: true,
  });

  assert.equal(result.report.ok, false);
  assert(result.report.failed_gate_ids.includes('supported_command_latency_trend_regression'));
}

function testReleaseVerdictAggregation() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'release-verdict-test-'));
  writeJson(path.join(root, 'policy.json'), {
    release_verdict: {
      required_gate_artifacts: {
        release_policy_gate: 'artifacts/release_policy_gate.json',
        'ops:production-topology:gate': 'artifacts/topology.json',
        'ops:stateful-upgrade-rollback:gate': 'artifacts/state.json',
        'ops:assimilation:v1:support:guard': 'artifacts/assimilation.json',
        'ops:release-blockers:gate': 'artifacts/blockers.json',
        'ops:release-hardening-window:guard': 'artifacts/hardening.json',
        'ops:release:scorecard:gate': 'artifacts/scorecard.json',
        'ops:production-closure:gate': 'artifacts/closure.json',
        'ops:release:rc-rehearsal': 'artifacts/rc.json',
      },
      checksum_artifact_paths: [
        'artifacts/release_policy_gate.json',
        'artifacts/scorecard.json',
        'artifacts/closure.json',
        'artifacts/rc.json',
      ],
    },
  });
  writeJson(path.join(root, 'artifacts/release_policy_gate.json'), { ok: true, strict: true });
  writeJson(path.join(root, 'artifacts/topology.json'), { ok: true, supported_production_topology: true, degraded_flags: [] });
  writeJson(path.join(root, 'artifacts/state.json'), { ok: true });
  writeJson(path.join(root, 'artifacts/assimilation.json'), { ok: true });
  writeJson(path.join(root, 'artifacts/blockers.json'), { ok: true });
  writeJson(path.join(root, 'artifacts/hardening.json'), { ok: true });
  writeJson(path.join(root, 'artifacts/scorecard.json'), { ok: true, strict: true });
  writeJson(path.join(root, 'artifacts/closure.json'), { strict: true, summary: { pass: true } });
  writeJson(path.join(root, 'artifacts/rc.json'), {
    ok: true,
    strict: true,
    summary: {
      candidate_ready: true,
      required_steps_satisfied: true,
    },
    steps: [
      { gate_id: 'release_policy_gate', ok: true },
      { gate_id: 'ops:production-topology:gate', ok: true },
      { gate_id: 'ops:stateful-upgrade-rollback:gate', ok: true },
      { gate_id: 'ops:assimilation:v1:support:guard', ok: true },
      { gate_id: 'ops:release-blockers:gate', ok: true },
      { gate_id: 'ops:release-hardening-window:guard', ok: true },
      { gate_id: 'ops:release:scorecard:gate', ok: true },
      { gate_id: 'ops:production-closure:gate', ok: true },
    ],
  });

  const pass = buildVerdictReport({
    strict: true,
    rootPath: root,
    out: 'artifacts/release_verdict.json',
    policyPath: 'policy.json',
  });
  assert.equal(pass.report.ok, true);
  assert.equal(typeof pass.report.verdict_checksum, 'string');
  assert.equal(pass.report.verdict_checksum.length, 64);

  writeJson(path.join(root, 'artifacts/closure.json'), { strict: false, summary: { pass: true } });
  const fail = buildVerdictReport({
    strict: true,
    rootPath: root,
    out: 'artifacts/release_verdict.json',
    policyPath: 'policy.json',
  });
  assert.equal(fail.report.ok, false);
  assert(fail.report.failed_ids.includes('release_gate_strict_artifact:ops:production-closure:gate'));
}

function main() {
  testScorecardTrendRegression();
  testReleaseVerdictAggregation();
  console.log(JSON.stringify({ ok: true, type: 'release_hardening_verdict_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
