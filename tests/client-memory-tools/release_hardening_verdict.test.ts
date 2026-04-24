#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const childProcess = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

require('../../client/runtime/lib/ts_bootstrap.ts').installTsRequireHook();

const { buildReport: buildScorecardReport } = require('../../tests/tooling/scripts/ci/release_scorecard_generate.ts');
const { buildReport: buildVerdictReport } = require('../../tests/tooling/scripts/ci/release_verdict_gate.ts');
const { buildReport: buildLegacyRunnerGuardReport } = require('../../tests/tooling/scripts/ci/legacy_process_runner_release_guard.ts');
const { buildReport: buildProductionStatusReport } = require('../../tests/tooling/scripts/ops/production_status.ts');

function writeJson(filePath, payload) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function withEnv(key, value, fn) {
  const previous = process.env[key];
  if (value == null) delete process.env[key];
  else process.env[key] = value;
  try {
    return fn();
  } finally {
    if (previous == null) delete process.env[key];
    else process.env[key] = previous;
  }
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
  writeJson(path.join(root, 'boundedness.json'), { ok: true, summary: { failed_count: 0, warning_count: 0 } });
  writeJson(path.join(root, 'node_critical.json'), {
    ok: true,
    summary: {
      operator_critical_priority_one_missing_rust_count: 0,
      migration_overdue_count: 0,
      ts_confinement_violation_count: 0,
    },
  });
  writeJson(path.join(root, 'benchmark.json'), {
    projects: {
      Infring: {
        kernel_shared_workload_ops_per_sec: 1000,
        rich_end_to_end_command_path_ops_per_sec: 100,
      },
    },
  });
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
    nodeCriticalPath: 'node_critical.json',
    topologyPath: 'topology.json',
    stateCompatPath: 'state.json',
    blockersPath: 'blockers.json',
    closurePath: 'closure.json',
    hardeningPath: 'hardening.json',
    boundednessReleaseGatePath: 'boundedness.json',
    ipcSoakPath: 'ipc.json',
    drPath: 'dr.json',
    benchmarkPath: 'benchmark.json',
    baselinePath: 'baseline.json',
    baselineTag: 'v0.3.10-alpha',
    requireBaseline: true,
    requireReleaseArtifacts: true,
  });

  assert.equal(result.report.ok, false);
  assert(result.report.failed_gate_ids.includes('supported_command_latency_trend_regression'));
}

function testScorecardPrebundleSkipsFinalSealDependencies() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'release-scorecard-prebundle-test-'));
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
    incident_truth_package: { ready: false, failed_checks: [{ id: 'pending_final_seal' }] },
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
  writeJson(path.join(root, 'closure.json'), { ok: false, summary: { pass: false }, failed_ids: ['support_bundle_incident_truth_package'] });
  writeJson(path.join(root, 'hardening.json'), { ok: true, active: false, violations: [] });
  writeJson(path.join(root, 'boundedness.json'), { ok: true, summary: { failed_count: 0, warning_count: 0 } });
  writeJson(path.join(root, 'node_critical.json'), {
    ok: true,
    summary: {
      operator_critical_priority_one_missing_rust_count: 0,
      migration_overdue_count: 0,
      ts_confinement_violation_count: 0,
    },
  });
  writeJson(path.join(root, 'benchmark.json'), {
    projects: {
      Infring: {
        kernel_shared_workload_ops_per_sec: 1000,
        rich_end_to_end_command_path_ops_per_sec: 100,
        median_user_workload_latency_ms: 100,
      },
    },
  });
  writeJson(path.join(root, 'ipc.json'), { rows: [{ ok: true }, { ok: true }, { ok: true }] });
  writeJson(path.join(root, 'dr.json'), { observed_rto_minutes: 8, observed_rpo_hours: 2 });

  const result = buildScorecardReport({
    strict: true,
    stage: 'prebundle',
    rootPath: root,
    outPath: 'out.json',
    semverPath: 'semver.json',
    commitLintPath: 'commit.json',
    policyPath: 'policy.json',
    canaryPath: 'canary.json',
    changelogPath: 'missing.md',
    closurePolicyPath: 'closure_policy.json',
    supportBundlePath: 'support_bundle.json',
    topologyPath: 'topology.json',
    stateCompatPath: 'state.json',
    blockersPath: 'blockers.json',
    closurePath: 'closure.json',
    hardeningPath: 'hardening.json',
    boundednessReleaseGatePath: 'boundedness.json',
    nodeCriticalPath: 'node_critical.json',
    ipcSoakPath: 'ipc.json',
    drPath: 'dr.json',
    benchmarkPath: 'benchmark.json',
    baselinePath: '',
    baselineTag: '',
    requireBaseline: false,
    requireReleaseArtifacts: false,
  });

  assert.equal(result.report.ok, true);
  assert.equal(result.report.stage, 'prebundle');
  const closureGate = result.report.gates.find((row) => row.id === 'production_closure_gate');
  const bundleGate = result.report.gates.find((row) => row.id === 'support_bundle_incident_truth_package');
  assert.equal(closureGate.ok, true);
  assert.match(String(closureGate.detail), /stage=prebundle/);
  assert.equal(bundleGate.ok, true);
  assert.match(String(bundleGate.detail), /stage=prebundle/);
}

function testClosurePrebundleSkipsFinalSealDependencies() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'release-closure-prebundle-test-'));
  writeJson(path.join(root, 'client/runtime/config/production_readiness_closure_policy.json'), {
    required_files: [],
    required_package_scripts: [],
    required_ci_invocations: [],
    required_verify_invocations: [],
    required_verify_profile_gate_ids: {},
    required_readme_markers: [],
    smoke_scripts: [],
    numeric_thresholds: {
      ipc_success_rate_min: 0.95,
      receipt_completeness_rate_min: 1,
      supported_command_latency_ms_max: 2500,
      recovery_rto_minutes_max: 30,
      recovery_rpo_hours_max: 24,
    },
    release_candidate_rehearsal: {
      required_step_gate_ids: ['audit:shell-layer-boundary'],
    },
    release_verdict: {
      required_gate_artifacts: {
        'ops:layer2:parity:guard': 'core/local/artifacts/layer2_lane_parity_guard_current.json',
        'ops:layer2:receipt:replay': 'core/local/artifacts/layer2_receipt_replay_current.json',
        'ops:trusted-core:report': 'core/local/artifacts/runtime_trusted_core_report_current.json',
        'ops:release:proof-pack': 'core/local/artifacts/release_proof_pack_current.json',
      },
    },
    standing_regression_guards: {
      shell_authority_gate_id: 'audit:shell-layer-boundary',
    },
  });
  fs.mkdirSync(path.join(root, 'tests/tooling/config'), { recursive: true });
  fs.writeFileSync(
    path.join(root, 'tests/tooling/config/release_gates.yaml'),
    [
      'version: 1',
      '  rich:',
      '      synthetic_required: 1',
      '      empirical_required: 1',
      '      empirical_min_sample_points: 1',
      '      baseline_pass_ratio_min: 1',
      '      fail_closed_ratio_min: 1',
      '      graduation_ratio_min: 1',
      '      workflow_unexpected_state_loop_max: 0',
      '      automatic_tool_trigger_events_max: 0',
      '      file_tool_route_misdirection_max: 0',
      '  pure:',
      '      synthetic_required: 1',
      '      empirical_required: 1',
      '      empirical_min_sample_points: 1',
      '      baseline_pass_ratio_min: 1',
      '      fail_closed_ratio_min: 1',
      '      graduation_ratio_min: 1',
      '      workflow_unexpected_state_loop_max: 0',
      '      automatic_tool_trigger_events_max: 0',
      '      file_tool_route_misdirection_max: 0',
      '  tiny-max:',
      '      synthetic_required: 1',
      '      empirical_required: 1',
      '      empirical_min_sample_points: 1',
      '      baseline_pass_ratio_min: 1',
      '      fail_closed_ratio_min: 1',
      '      graduation_ratio_min: 1',
      '      workflow_unexpected_state_loop_max: 0',
      '      automatic_tool_trigger_events_max: 0',
      '      file_tool_route_misdirection_max: 0',
      '  sentinel:',
      '',
    ].join('\n'),
    'utf8',
  );
  writeJson(path.join(root, 'scorecard.json'), {
    ok: true,
    thresholds: {
      ipc_success_rate: 1,
      receipt_completeness_rate: 1,
      max_command_latency_ms: 900,
      observed_rto_minutes: 10,
      observed_rpo_hours: 2,
    },
    gates: [
      { id: 'ipc_success_rate_threshold', ok: true },
      { id: 'receipt_completeness_threshold', ok: true },
      { id: 'supported_command_latency_threshold', ok: true },
      { id: 'recovery_rto_threshold', ok: true },
      { id: 'recovery_rpo_threshold', ok: true },
    ],
  });
  writeJson(path.join(root, 'support_bundle.json'), {
    incident_truth_package: { ready: false, failed_checks: [{ id: 'pending_final_seal' }] },
  });
  writeJson(path.join(root, 'topology.json'), {
    ok: true,
    supported_production_topology: true,
    support_level: 'production_supported',
    degraded_flags: [],
  });
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
  writeJson(path.join(root, 'shell_boundary.json'), {
    ok: true,
    summary: { pass: true, violation_count: 0 },
  });
  writeJson(path.join(root, 'rc.json'), {
    ok: false,
    summary: { failed_count: 1 },
    steps: [],
  });

  const previousCwd = process.cwd();
  process.chdir(root);
  try {
    const modulePath = require.resolve('../../tests/tooling/scripts/ci/production_readiness_closure_gate.ts');
    delete require.cache[modulePath];
    const { buildReport: buildClosureReport } = require(modulePath);
    const report = buildClosureReport({
      strict: true,
      out: 'closure.json',
      runSmoke: false,
      stage: 'prebundle',
      supportBundlePath: path.join(root, 'support_bundle.json'),
      scorecardPath: path.join(root, 'scorecard.json'),
      topologyPath: path.join(root, 'topology.json'),
      stateCompatPath: path.join(root, 'state.json'),
      rcRehearsalPath: path.join(root, 'rc.json'),
      shellBoundaryPath: path.join(root, 'shell_boundary.json'),
    });
    assert.equal(report.summary.pass, true);
    assert.equal(report.stage, 'prebundle');
    const rcGate = report.checks.find((row) => row.id === 'release_candidate_rehearsal_completed');
    const bundleGate = report.checks.find((row) => row.id === 'support_bundle_incident_truth_package');
    const boundaryGate = report.checks.find((row) => row.id === 'shell_authority_regression_guard');
    assert.equal(rcGate.ok, true);
    assert.match(String(rcGate.detail), /stage=prebundle/);
    assert.equal(bundleGate.ok, true);
    assert.match(String(bundleGate.detail), /stage=prebundle/);
    assert.equal(boundaryGate.ok, true);
  } finally {
    process.chdir(previousCwd);
  }
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

function testHardeningWindowDetectsNewCommandsAndCoreFiles() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'release-hardening-window-test-'));
  fs.mkdirSync(path.join(root, 'client/runtime/config'), { recursive: true });
  fs.mkdirSync(path.join(root, 'tests/tooling/config'), { recursive: true });
  fs.mkdirSync(path.join(root, 'core/existing'), { recursive: true });
  fs.writeFileSync(
    path.join(root, 'client/runtime/config/release_hardening_window_policy.json'),
    JSON.stringify(
      {
        activation_env: 'INFRING_RELEASE_HARDENING_WINDOW',
        default_base_ref: 'HEAD~1',
        blocked_prefixes: [],
        blocked_added_file_prefixes: ['core/'],
        block_new_package_scripts: true,
        block_new_tooling_gate_ids: true,
      },
      null,
      2,
    ),
  );
  fs.writeFileSync(path.join(root, 'package.json'), JSON.stringify({ scripts: { keep: 'echo ok' } }, null, 2));
  fs.writeFileSync(
    path.join(root, 'tests/tooling/config/tooling_gate_registry.json'),
    JSON.stringify({ version: '1.0', gates: { keep: { owner: 'ops', description: 'x', script: 'keep' } } }, null, 2),
  );
  fs.writeFileSync(path.join(root, 'core/existing/file.txt'), 'base\n');
  childProcess.execFileSync('git', ['init'], { cwd: root });
  childProcess.execFileSync('git', ['config', 'user.email', 'test@example.com'], { cwd: root });
  childProcess.execFileSync('git', ['config', 'user.name', 'Test'], { cwd: root });
  childProcess.execFileSync('git', ['add', '.'], { cwd: root });
  childProcess.execFileSync('git', ['commit', '-m', 'base'], { cwd: root });
  fs.mkdirSync(path.join(root, 'core/new_area'), { recursive: true });
  fs.writeFileSync(path.join(root, 'core/new_area/new.rs'), 'pub fn noop() {}\n');
  fs.writeFileSync(
    path.join(root, 'package.json'),
    JSON.stringify({ scripts: { keep: 'echo ok', 'ops:new-command': 'echo nope' } }, null, 2),
  );
  fs.writeFileSync(
    path.join(root, 'tests/tooling/config/tooling_gate_registry.json'),
    JSON.stringify(
      {
        version: '1.0',
        gates: {
          keep: { owner: 'ops', description: 'x', script: 'keep' },
          'ops:new-gate': { owner: 'ops', description: 'y', script: 'ops:new-command' },
        },
      },
      null,
      2,
    ),
  );
  childProcess.execFileSync('git', ['add', '.'], { cwd: root });
  childProcess.execFileSync('git', ['commit', '-m', 'hardening drift'], { cwd: root });
  const previousCwd = process.cwd();
  process.chdir(root);
  try {
    withEnv('INFRING_RELEASE_HARDENING_WINDOW', '1', () => {
      const modulePath = require.resolve('../../tests/tooling/scripts/ci/release_hardening_window_guard.ts');
      delete require.cache[modulePath];
      const { buildReport: buildHardeningWindowReport } = require(modulePath);
      const report = buildHardeningWindowReport();
      assert.equal(report.ok, false);
      assert(report.added_file_violations.includes('core/new_area/new.rs'));
      assert(report.new_package_script_violations.includes('ops:new-command'));
      assert(report.new_tooling_gate_violations.includes('ops:new-gate'));
    });
  } finally {
    process.chdir(previousCwd);
  }
}

function testLegacyRunnerDeadlineEnforcedAfterCutoff() {
  const report = withEnv('INFRING_LEGACY_RUNNER_GUARD_TODAY', '2026-05-16', () =>
    buildLegacyRunnerGuardReport(),
  );
  assert.equal(report.ok, false);
  assert(report.failed.includes('legacy_runner_deleted_by_cutoff'));
}

function testProductionStatusReportShape() {
  const result = buildProductionStatusReport({
    strict: false,
    out: path.join(os.tmpdir(), 'production_status_test.json'),
  });
  assert.equal(result.report.type, 'production_status');
  assert.equal(typeof result.report.summary.release_ready, 'boolean');
  assert.equal(Array.isArray(result.report.degraded_flags), true);
  assert.equal(typeof result.report.version.runtime_version_matches_repo_tag, 'boolean');
}

function main() {
  testScorecardTrendRegression();
  testScorecardPrebundleSkipsFinalSealDependencies();
  testClosurePrebundleSkipsFinalSealDependencies();
  testReleaseVerdictAggregation();
  testHardeningWindowDetectsNewCommandsAndCoreFiles();
  testLegacyRunnerDeadlineEnforcedAfterCutoff();
  testProductionStatusReportShape();
  console.log(JSON.stringify({ ok: true, type: 'release_hardening_verdict_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
