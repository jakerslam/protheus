#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';

function cleanText(value: unknown, maxLen = 2000): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function parseArgs(argv: string[]) {
  const out = {
    strict: false,
    stage: 'prebundle' as 'prebundle' | 'final',
    outPath: 'client/runtime/local/state/release/scorecard/release_scorecard.json',
    semverPath: '/tmp/release-plan.json',
    commitLintPath: 'core/local/artifacts/conventional_commit_gate_current.json',
    policyPath: 'core/local/artifacts/release_policy_gate_current.json',
    canaryPath: 'core/local/artifacts/release_canary_gate_current.json',
    changelogPath: 'client/runtime/local/state/release/CHANGELOG.auto.md',
    closurePolicyPath: 'client/runtime/config/production_readiness_closure_policy.json',
    supportBundlePath: 'core/local/artifacts/support_bundle_latest.json',
    nodeCriticalPath: 'core/local/artifacts/node_critical_path_inventory_current.json',
    topologyPath: 'core/local/artifacts/production_topology_diagnostic_current.json',
    stateCompatPath: 'core/local/artifacts/stateful_upgrade_rollback_gate_current.json',
    blockersPath: 'core/local/artifacts/release_blocker_rubric_current.json',
    closurePath: 'core/local/artifacts/production_readiness_closure_gate_current.json',
    hardeningPath: 'core/local/artifacts/release_hardening_window_guard_current.json',
    boundednessReleaseGatePath: 'core/local/artifacts/runtime_boundedness_release_gate_current.json',
    ipcSoakPath: 'local/state/ops/ops_ipc_bridge_stability_soak/latest.json',
    drPath: 'local/state/ops/dr_gameday/latest.json',
    benchmarkPath: 'docs/client/reports/benchmark_matrix_run_latest.json',
    benchmarkBaselinePath: '',
    baselinePath: '',
    baselineTag: '',
    requireBaseline: false,
    requireReleaseArtifacts: false,
    rootPath: '',
  };
  for (const tokenRaw of argv) {
    const token = cleanText(tokenRaw, 400);
    if (!token) continue;
    if (token.startsWith('--strict=')) {
      out.strict = ['1', 'true', 'yes', 'on'].includes(cleanText(token.slice(9), 40).toLowerCase());
    }
    else if (token.startsWith('--stage=')) {
      const stage = cleanText(token.slice(8), 40).toLowerCase();
      out.stage = stage === 'final' ? 'final' : 'prebundle';
    }
    if (token.startsWith('--out=')) out.outPath = cleanText(token.slice(6), 400);
    else if (token.startsWith('--semver=')) out.semverPath = cleanText(token.slice(9), 400);
    else if (token.startsWith('--commit-lint=')) out.commitLintPath = cleanText(token.slice(14), 400);
    else if (token.startsWith('--policy=')) out.policyPath = cleanText(token.slice(9), 400);
    else if (token.startsWith('--canary=')) out.canaryPath = cleanText(token.slice(9), 400);
    else if (token.startsWith('--changelog=')) out.changelogPath = cleanText(token.slice(12), 400);
    else if (token.startsWith('--closure-policy=')) out.closurePolicyPath = cleanText(token.slice(17), 400);
    else if (token.startsWith('--support-bundle=')) out.supportBundlePath = cleanText(token.slice(17), 400);
    else if (token.startsWith('--node-critical-path=')) out.nodeCriticalPath = cleanText(token.slice(21), 400);
    else if (token.startsWith('--topology=')) out.topologyPath = cleanText(token.slice(11), 400);
    else if (token.startsWith('--state-compat=')) out.stateCompatPath = cleanText(token.slice(15), 400);
    else if (token.startsWith('--blockers=')) out.blockersPath = cleanText(token.slice(11), 400);
    else if (token.startsWith('--closure=')) out.closurePath = cleanText(token.slice(10), 400);
    else if (token.startsWith('--hardening=')) out.hardeningPath = cleanText(token.slice(12), 400);
    else if (token.startsWith('--boundedness-release-gate=')) out.boundednessReleaseGatePath = cleanText(token.slice(27), 400);
    else if (token.startsWith('--ipc-soak=')) out.ipcSoakPath = cleanText(token.slice(11), 400);
    else if (token.startsWith('--dr=')) out.drPath = cleanText(token.slice(5), 400);
    else if (token.startsWith('--benchmark=')) out.benchmarkPath = cleanText(token.slice(12), 400);
    else if (token.startsWith('--benchmark-baseline=')) out.benchmarkBaselinePath = cleanText(token.slice(21), 400);
    else if (token.startsWith('--baseline=')) out.baselinePath = cleanText(token.slice(11), 400);
    else if (token.startsWith('--baseline-tag=')) out.baselineTag = cleanText(token.slice(15), 400);
    else if (token.startsWith('--require-baseline=')) {
      out.requireBaseline = ['1', 'true', 'yes', 'on'].includes(
        cleanText(token.slice(19), 40).toLowerCase(),
      );
    }
    else if (token.startsWith('--require-release-artifacts=')) {
      out.requireReleaseArtifacts = ['1', 'true', 'yes', 'on'].includes(
        cleanText(token.slice(28), 40).toLowerCase(),
      );
    }
  }
  return out;
}

function readJsonMaybe(filePath: string): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function readJsonFirst(paths: string[]): any {
  for (const filePath of paths) {
    if (!filePath) continue;
    const parsed = readJsonMaybe(filePath);
    if (parsed && typeof parsed === 'object') return parsed;
  }
  return null;
}

function resolveMaybe(root: string, maybePath: string): string {
  if (path.isAbsolute(maybePath)) return maybePath;
  return path.resolve(root, maybePath);
}

function gateRow(id: string, ok: boolean, detail: string) {
  return { id, ok, detail };
}

function optionalGateRow(id: string, required: boolean, ok: boolean, detail: string) {
  return required ? gateRow(id, ok, detail) : gateRow(id, true, `optional:${detail}`);
}

function safeNumber(value: unknown, fallback = 0): number {
  const num = Number(value);
  return Number.isFinite(num) ? num : fallback;
}

function trendGateRow(
  id: string,
  current: number,
  baseline: number,
  mode: 'min' | 'max',
) {
  const ok = mode === 'min' ? current >= baseline : current <= baseline;
  return gateRow(
    id,
    ok,
    `value=${Number.isFinite(current) ? current : 'missing'};baseline=${Number.isFinite(baseline) ? baseline : 'missing'};mode=${mode}`,
  );
}

function releaseChannel(raw: unknown): 'alpha' | 'beta' | 'stable' {
  const normalized = cleanText(raw ?? '', 40).toLowerCase();
  if (normalized === 'alpha' || normalized === 'beta' || normalized === 'stable') {
    return normalized;
  }
  return 'stable';
}

function findNumericByKey(input: any, key: string, depth = 0): number {
  if (!input || depth > 6) return Number.NaN;
  if (typeof input !== 'object') return Number.NaN;
  if (Object.prototype.hasOwnProperty.call(input, key)) {
    const value = Number((input as any)[key]);
    if (Number.isFinite(value)) return value;
  }
  if (Array.isArray(input)) {
    for (const row of input) {
      const found = findNumericByKey(row, key, depth + 1);
      if (Number.isFinite(found)) return found;
    }
    return Number.NaN;
  }
  for (const value of Object.values(input)) {
    const found = findNumericByKey(value, key, depth + 1);
    if (Number.isFinite(found)) return found;
  }
  return Number.NaN;
}

function firstFinite(values: number[], fallback = Number.NaN): number {
  for (const value of values) {
    if (Number.isFinite(value)) return value;
  }
  return fallback;
}

function benchmarkTrendBudgetGate(
  id: string,
  current: number,
  baseline: number,
  direction: 'higher_is_better' | 'lower_is_better',
  maxRegressionPct: number,
) {
  if (!Number.isFinite(current) || !Number.isFinite(baseline) || baseline <= 0) {
    return gateRow(id, true, 'optional:baseline_or_current_missing');
  }
  const budget = Math.max(0, maxRegressionPct);
  if (direction === 'higher_is_better') {
    const minAllowed = baseline * (1 - budget);
    return gateRow(
      id,
      current >= minAllowed,
      `current=${current};baseline=${baseline};min_allowed=${minAllowed};max_regression_pct=${budget}`,
    );
  }
  const maxAllowed = baseline * (1 + budget);
  return gateRow(
    id,
    current <= maxAllowed,
    `current=${current};baseline=${baseline};max_allowed=${maxAllowed};max_regression_pct=${budget}`,
  );
}

function buildReport(args = parseArgs(process.argv.slice(2))) {
  const normalizedArgs = { ...parseArgs([]), ...args };
  const root = path.resolve(normalizedArgs.rootPath || path.resolve(__dirname, '../../../..'));
  const semverPath = resolveMaybe(root, normalizedArgs.semverPath);
  const commitLintPath = resolveMaybe(root, normalizedArgs.commitLintPath);
  const policyPath = resolveMaybe(root, normalizedArgs.policyPath);
  const canaryPath = resolveMaybe(root, normalizedArgs.canaryPath);
  const changelogPath = resolveMaybe(root, normalizedArgs.changelogPath);
  const closurePolicyPath = resolveMaybe(root, normalizedArgs.closurePolicyPath);
  const supportBundlePath = resolveMaybe(root, normalizedArgs.supportBundlePath);
  const nodeCriticalPath = resolveMaybe(root, normalizedArgs.nodeCriticalPath);
  const topologyPath = resolveMaybe(root, normalizedArgs.topologyPath);
  const stateCompatPath = resolveMaybe(root, normalizedArgs.stateCompatPath);
  const blockersPath = resolveMaybe(root, normalizedArgs.blockersPath);
  const closurePath = resolveMaybe(root, normalizedArgs.closurePath);
  const hardeningPath = resolveMaybe(root, normalizedArgs.hardeningPath);
  const boundednessReleaseGatePath = resolveMaybe(root, normalizedArgs.boundednessReleaseGatePath);
  const ipcSoakPath = resolveMaybe(root, normalizedArgs.ipcSoakPath);
  const drPath = resolveMaybe(root, normalizedArgs.drPath);
  const benchmarkPath = resolveMaybe(root, normalizedArgs.benchmarkPath);
  const benchmarkBaselinePath = normalizedArgs.benchmarkBaselinePath
    ? resolveMaybe(root, normalizedArgs.benchmarkBaselinePath)
    : '';
  const baselinePath = normalizedArgs.baselinePath ? resolveMaybe(root, normalizedArgs.baselinePath) : '';
  const ipcSoakFallbackPath = path.join(root, 'artifacts', 'ops_ipc_bridge_stability_soak_report_latest.json');

  const semver = readJsonMaybe(semverPath) ?? {};
  const commitLint = readJsonMaybe(commitLintPath) ?? {};
  const policy = readJsonMaybe(policyPath) ?? {};
  const canary = readJsonMaybe(canaryPath) ?? {};
  const closurePolicy = readJsonMaybe(closurePolicyPath) ?? {};
  const supportBundle = readJsonMaybe(supportBundlePath) ?? {};
  const nodeCriticalPathInventory = readJsonMaybe(nodeCriticalPath) ?? {};
  const topology = readJsonMaybe(topologyPath) ?? {};
  const stateCompat = readJsonMaybe(stateCompatPath) ?? {};
  const blockers = readJsonMaybe(blockersPath) ?? {};
  const closure = readJsonMaybe(closurePath) ?? {};
  const hardening = readJsonMaybe(hardeningPath) ?? {};
  const boundednessReleaseGate = readJsonMaybe(boundednessReleaseGatePath) ?? {};
  const ipcSoak = readJsonFirst([ipcSoakPath, ipcSoakFallbackPath]) ?? {};
  const dr = readJsonMaybe(drPath) ?? {};
  const benchmark = readJsonMaybe(benchmarkPath) ?? {};
  const benchmarkBaseline = benchmarkBaselinePath ? readJsonMaybe(benchmarkBaselinePath) ?? {} : null;
  const baselineScorecard = baselinePath ? readJsonMaybe(baselinePath) ?? {} : null;
  const channel = releaseChannel(semver?.release_channel);
  const requireReleaseArtifacts = Boolean(normalizedArgs.requireReleaseArtifacts);
  const requireBaseline = Boolean(normalizedArgs.requireBaseline);
  const finalStage = normalizedArgs.stage === 'final';

  const changelogExists = fs.existsSync(changelogPath);
  const canaryOk = canary?.ok === true;
  const canaryRequired = requireReleaseArtifacts && channel === 'stable';
  const canaryGateOk = canaryRequired ? canaryOk : true;
  const thresholds = closurePolicy?.numeric_thresholds ?? {};
  const benchmarkBudgets = closurePolicy?.benchmark_regression_budgets ?? {};
  const ipcRows = Array.isArray(ipcSoak?.rows) ? ipcSoak.rows : [];
  const ipcSuccessRate =
    ipcRows.length === 0 ? 0 : ipcRows.filter((row: any) => row && row.ok === true).length / ipcRows.length;
  const receiptCompleteness = safeNumber(supportBundle?.metrics?.receipt_completeness_rate, 0);
  const maxCommandLatencyMs = safeNumber(
    supportBundle?.metrics?.supported_command_latency_ms,
    safeNumber(supportBundle?.metrics?.max_command_latency_ms, Number.POSITIVE_INFINITY),
  );
  const closureCommandLatencyMs = safeNumber(
    supportBundle?.metrics?.max_command_latency_ms,
    Number.POSITIVE_INFINITY,
  );
  const observedRtoMinutes = safeNumber(dr?.observed_rto_minutes, Number.POSITIVE_INFINITY);
  const observedRpoHours = safeNumber(dr?.observed_rpo_hours, Number.POSITIVE_INFINITY);
  const nodeCriticalInventoryOk = nodeCriticalPathInventory?.ok === true;
  const nodeCriticalPriorityOneMissingRustCount = safeNumber(
    nodeCriticalPathInventory?.summary?.operator_critical_priority_one_missing_rust_count,
    Number.POSITIVE_INFINITY,
  );
  const nodeCriticalMigrationOverdueCount = safeNumber(
    nodeCriticalPathInventory?.summary?.migration_overdue_count,
    Number.POSITIVE_INFINITY,
  );
  const nodeCriticalTsConfinementViolationCount = safeNumber(
    nodeCriticalPathInventory?.summary?.ts_confinement_violation_count,
    Number.POSITIVE_INFINITY,
  );
  const baselineAvailable = Boolean(baselineScorecard && typeof baselineScorecard === 'object');
  const baselineThresholds = baselineAvailable ? baselineScorecard?.thresholds ?? {} : {};
  const baselineTag = cleanText(normalizedArgs.baselineTag || baselineScorecard?.tag || 'none', 120);
  const trendGates = baselineAvailable
    ? [
        trendGateRow(
          'ipc_success_rate_trend_regression',
          Number(ipcSuccessRate.toFixed(4)),
          safeNumber(baselineThresholds?.ipc_success_rate, Number.NaN),
          'min',
        ),
        trendGateRow(
          'receipt_completeness_trend_regression',
          Number(receiptCompleteness.toFixed(4)),
          safeNumber(baselineThresholds?.receipt_completeness_rate, Number.NaN),
          'min',
        ),
        trendGateRow(
          'supported_command_latency_trend_regression',
          maxCommandLatencyMs,
          safeNumber(baselineThresholds?.max_command_latency_ms, Number.NaN),
          'max',
        ),
        trendGateRow(
          'recovery_rto_trend_regression',
          observedRtoMinutes,
          safeNumber(baselineThresholds?.observed_rto_minutes, Number.NaN),
          'max',
        ),
        trendGateRow(
          'recovery_rpo_trend_regression',
          observedRpoHours,
          safeNumber(baselineThresholds?.observed_rpo_hours, Number.NaN),
          'max',
        ),
      ]
    : [];

  const benchmarkProjectInfring = benchmark?.projects?.Infring ?? {};
  const microkernelSharedOps = firstFinite([
    safeNumber(benchmarkProjectInfring?.kernel_shared_workload_ops_per_sec, Number.NaN),
    safeNumber(benchmark?.kernel_shared_workload_ops_per_sec, Number.NaN),
    findNumericByKey(benchmark, 'kernel_shared_workload_ops_per_sec'),
    safeNumber(benchmarkProjectInfring?.tasks_per_sec, Number.NaN),
  ]);
  const governedCommandPathOps = firstFinite([
    safeNumber(benchmarkProjectInfring?.rich_end_to_end_command_path_ops_per_sec, Number.NaN),
    safeNumber(benchmark?.rich_end_to_end_command_path_ops_per_sec, Number.NaN),
    findNumericByKey(benchmark, 'rich_end_to_end_command_path_ops_per_sec'),
  ]);
  const readinessLatencyMs = firstFinite([
    safeNumber(benchmarkProjectInfring?.rich_cold_start_total_ms, Number.NaN),
    safeNumber(benchmarkProjectInfring?.cold_start_ms, Number.NaN),
    safeNumber(benchmark?.cold_start_ms, Number.NaN),
    findNumericByKey(benchmark, 'rich_cold_start_total_ms'),
    findNumericByKey(benchmark, 'cold_start_ms'),
  ]);
  const userWorkloadLatencyMs = maxCommandLatencyMs;

  const benchmarkClasses = {
    microkernel_shared_workload: {
      metric: 'kernel_shared_workload_ops_per_sec',
      direction: 'higher_is_better',
      value: Number.isFinite(microkernelSharedOps) ? microkernelSharedOps : null,
      source_path: path.relative(root, benchmarkPath),
    },
    governed_command_path: {
      metric: 'rich_end_to_end_command_path_ops_per_sec',
      direction: 'higher_is_better',
      value: Number.isFinite(governedCommandPathOps) ? governedCommandPathOps : null,
      source_path: path.relative(root, benchmarkPath),
    },
    realistic_user_workload: {
      metric: 'supported_command_latency_ms',
      direction: 'lower_is_better',
      value: Number.isFinite(userWorkloadLatencyMs) ? userWorkloadLatencyMs : null,
      readiness_latency_ms: Number.isFinite(readinessLatencyMs) ? readinessLatencyMs : null,
      source_path: path.relative(root, supportBundlePath),
    },
  };

  const benchmarkBaselineClasses = benchmarkBaseline?.projects?.Infring
    ? {
        microkernel_shared_workload: safeNumber(
          benchmarkBaseline?.projects?.Infring?.kernel_shared_workload_ops_per_sec,
          Number.NaN,
        ),
        governed_command_path: safeNumber(
          benchmarkBaseline?.projects?.Infring?.rich_end_to_end_command_path_ops_per_sec,
          Number.NaN,
        ),
        readiness_latency_ms: firstFinite([
          safeNumber(benchmarkBaseline?.projects?.Infring?.rich_cold_start_total_ms, Number.NaN),
          safeNumber(benchmarkBaseline?.projects?.Infring?.cold_start_ms, Number.NaN),
        ]),
      }
    : baselineScorecard?.benchmark_classes
      ? {
          microkernel_shared_workload: safeNumber(
            baselineScorecard?.benchmark_classes?.microkernel_shared_workload?.value,
            Number.NaN,
          ),
          governed_command_path: safeNumber(
            baselineScorecard?.benchmark_classes?.governed_command_path?.value,
            Number.NaN,
          ),
          readiness_latency_ms: safeNumber(
            baselineScorecard?.benchmark_classes?.realistic_user_workload?.readiness_latency_ms,
            Number.NaN,
          ),
        }
      : {
          microkernel_shared_workload: Number.NaN,
          governed_command_path: Number.NaN,
          readiness_latency_ms: Number.NaN,
        };

  const benchmarkTrendGates = [
    benchmarkTrendBudgetGate(
      'benchmark_microkernel_shared_workload_regression_budget',
      microkernelSharedOps,
      safeNumber(benchmarkBaselineClasses?.microkernel_shared_workload, Number.NaN),
      'higher_is_better',
      safeNumber(benchmarkBudgets?.microkernel_shared_workload_pct_max, 0.1),
    ),
    benchmarkTrendBudgetGate(
      'benchmark_governed_command_path_regression_budget',
      governedCommandPathOps,
      safeNumber(benchmarkBaselineClasses?.governed_command_path, Number.NaN),
      'higher_is_better',
      safeNumber(benchmarkBudgets?.governed_command_path_pct_max, 0.1),
    ),
    benchmarkTrendBudgetGate(
      'benchmark_readiness_latency_regression_budget',
      readinessLatencyMs,
      safeNumber(benchmarkBaselineClasses?.readiness_latency_ms, Number.NaN),
      'lower_is_better',
      safeNumber(benchmarkBudgets?.readiness_latency_pct_max, 0.1),
    ),
  ];

  const liveChecks = stateCompat?.checks ?? {};
  const liveRehearsalOk =
    liveChecks.live_taskgroup_rehearsal_verified === true &&
    liveChecks.live_receipt_rehearsal_verified === true &&
    liveChecks.live_memory_surface_verified === true &&
    liveChecks.live_runtime_receipt_verified === true &&
    liveChecks.live_assimilation_contract_verified === true;
  const gates = [
    optionalGateRow(
      'semver_plan',
      requireReleaseArtifacts,
      !!semver && semver.ok === true && typeof semver.next_tag === 'string',
      `next_tag=${cleanText(semver?.next_tag ?? 'none', 120)}`
    ),
    optionalGateRow(
      'conventional_commit_lint',
      requireReleaseArtifacts,
      !!commitLint && (commitLint.ok === true || commitLint.strict === false),
      `invalid_count=${Number(commitLint?.invalid_count ?? 0)}`
    ),
    gateRow(
      'release_policy_gate',
      !!policy && policy.ok === true,
      `failed=${Array.isArray(policy?.failed) ? policy.failed.join(',') : 'none'}`
    ),
    optionalGateRow(
      'canary_rollback_gate',
      requireReleaseArtifacts,
      canaryGateOk,
      canaryRequired
        ? `required=true;canary_ok=${canaryOk}`
        : `required=false;canary_ok=${canaryOk}`
    ),
    gateRow(
      'production_closure_gate',
      !finalStage || closure?.summary?.pass === true || closure?.ok === true,
      finalStage
        ? `failed=${Array.isArray(closure?.failed_ids) ? closure.failed_ids.join(',') : 'none'}`
        : 'stage=prebundle;final_closure_not_required'
    ),
    gateRow(
      'production_topology_diagnostic',
      topology?.ok === true && topology?.supported_production_topology === true,
      `support_level=${cleanText(topology?.support_level ?? 'unknown', 80)}`
    ),
    gateRow(
      'stateful_upgrade_rollback_gate',
      stateCompat?.ok === true,
      `errors=${Array.isArray(stateCompat?.errors) ? stateCompat.errors.length : 0}`
    ),
    gateRow(
      'live_upgrade_rollback_rehearsal',
      liveRehearsalOk,
      `taskgroup=${liveChecks.live_taskgroup_rehearsal_verified === true};receipt=${liveChecks.live_receipt_rehearsal_verified === true};memory=${liveChecks.live_memory_surface_verified === true};runtime_receipt=${liveChecks.live_runtime_receipt_verified === true};assimilation=${liveChecks.live_assimilation_contract_verified === true}`
    ),
    gateRow(
      'release_blocker_rubric_gate',
      blockers?.ok === true,
      `open_release_blockers=${Array.isArray(blockers?.open_release_blockers) ? blockers.open_release_blockers.length : 0};budget_remaining=${safeNumber(blockers?.release_blocker_budget_remaining, -1)}`
    ),
    gateRow(
      'release_hardening_window_guard',
      hardening?.ok === true,
      `active=${hardening?.active === true};violations=${Array.isArray(hardening?.violations) ? hardening.violations.length : 0}`
    ),
    gateRow(
      'ipc_success_rate_threshold',
      ipcSuccessRate >= safeNumber(thresholds.ipc_success_rate_min, 0.95),
      `value=${ipcSuccessRate.toFixed(4)};min=${safeNumber(thresholds.ipc_success_rate_min, 0.95)}`
    ),
    gateRow(
      'receipt_completeness_threshold',
      receiptCompleteness >= safeNumber(thresholds.receipt_completeness_rate_min, 1),
      `value=${receiptCompleteness.toFixed(4)};min=${safeNumber(thresholds.receipt_completeness_rate_min, 1)}`
    ),
    gateRow(
      'supported_command_latency_threshold',
      maxCommandLatencyMs <= safeNumber(thresholds.supported_command_latency_ms_max, 2500),
      `value=${maxCommandLatencyMs};max=${safeNumber(thresholds.supported_command_latency_ms_max, 2500)}`
    ),
    gateRow(
      'recovery_rto_threshold',
      observedRtoMinutes <= safeNumber(thresholds.recovery_rto_minutes_max, 30),
      `value=${observedRtoMinutes};max=${safeNumber(thresholds.recovery_rto_minutes_max, 30)}`
    ),
    gateRow(
      'recovery_rpo_threshold',
      observedRpoHours <= safeNumber(thresholds.recovery_rpo_hours_max, 24),
      `value=${observedRpoHours};max=${safeNumber(thresholds.recovery_rpo_hours_max, 24)}`
    ),
    gateRow(
      'boundedness_release_gate',
      boundednessReleaseGate?.ok === true,
      `failed=${safeNumber(boundednessReleaseGate?.summary?.failed_count, Number.NaN)};warnings=${safeNumber(boundednessReleaseGate?.summary?.warning_count, Number.NaN)}`
    ),
    gateRow(
      'node_critical_path_inventory_gate',
      nodeCriticalInventoryOk,
      `path=${path.relative(root, nodeCriticalPath)};ok=${nodeCriticalInventoryOk}`,
    ),
    gateRow(
      'node_critical_priority_one_rust_target_coverage',
      nodeCriticalPriorityOneMissingRustCount === 0,
      `value=${nodeCriticalPriorityOneMissingRustCount};expected=0`,
    ),
    gateRow(
      'node_critical_migration_overdue_zero',
      nodeCriticalMigrationOverdueCount === 0,
      `value=${nodeCriticalMigrationOverdueCount};expected=0`,
    ),
    gateRow(
      'node_critical_ts_confinement_violations_zero',
      nodeCriticalTsConfinementViolationCount === 0,
      `value=${nodeCriticalTsConfinementViolationCount};expected=0`,
    ),
    gateRow(
      'benchmark_class_microkernel_shared_present',
      Number.isFinite(microkernelSharedOps) && microkernelSharedOps > 0,
      `value=${Number.isFinite(microkernelSharedOps) ? microkernelSharedOps : 'missing'}`
    ),
    gateRow(
      'benchmark_class_governed_command_path_present',
      Number.isFinite(governedCommandPathOps) && governedCommandPathOps > 0,
      `value=${Number.isFinite(governedCommandPathOps) ? governedCommandPathOps : 'missing'}`
    ),
    gateRow(
      'benchmark_class_user_workload_present',
      Number.isFinite(userWorkloadLatencyMs) && userWorkloadLatencyMs > 0,
      `value=${Number.isFinite(userWorkloadLatencyMs) ? userWorkloadLatencyMs : 'missing'}`
    ),
    gateRow(
      'support_bundle_incident_truth_package',
      !finalStage || supportBundle?.incident_truth_package?.ready === true,
      finalStage
        ? `failed_checks=${Array.isArray(supportBundle?.incident_truth_package?.failed_checks) ? supportBundle.incident_truth_package.failed_checks.length : 0}`
        : 'stage=prebundle;final_bundle_truth_not_required'
    ),
    optionalGateRow(
      'changelog_generated',
      requireReleaseArtifacts,
      changelogExists,
      `path=${path.relative(root, changelogPath)}`
    ),
    optionalGateRow(
      'previous_release_scorecard_baseline',
      requireBaseline,
      baselineAvailable,
      baselinePath ? `path=${path.relative(root, baselinePath)};tag=${baselineTag}` : 'missing',
    ),
    ...trendGates,
    ...benchmarkTrendGates,
  ];
  const overall = gates.every((row) => row.ok);
  const report = {
    ok: overall,
    type: 'release_scorecard',
    generated_at: new Date().toISOString(),
    strict: Boolean(normalizedArgs.strict),
    stage: normalizedArgs.stage,
    channel,
    tag: cleanText(semver?.next_tag ?? 'none', 120),
    version: cleanText(semver?.next_version ?? semver?.current_version ?? '0.0.0', 120),
    baseline: {
      required: requireBaseline,
      available: baselineAvailable,
      path: baselinePath ? path.relative(root, baselinePath) : '',
      tag: baselineTag,
      version: cleanText(baselineScorecard?.version ?? '', 120),
    },
    benchmark_baseline: {
      path: benchmarkBaselinePath ? path.relative(root, benchmarkBaselinePath) : '',
      available: Boolean(benchmarkBaseline && typeof benchmarkBaseline === 'object'),
      regression_budgets: {
        microkernel_shared_workload_pct_max: safeNumber(benchmarkBudgets?.microkernel_shared_workload_pct_max, 0.1),
        governed_command_path_pct_max: safeNumber(benchmarkBudgets?.governed_command_path_pct_max, 0.1),
        readiness_latency_pct_max: safeNumber(benchmarkBudgets?.readiness_latency_pct_max, 0.1),
      },
    },
    policy_thresholds: {
      ipc_success_rate_min: safeNumber(thresholds.ipc_success_rate_min, 0.95),
      receipt_completeness_rate_min: safeNumber(thresholds.receipt_completeness_rate_min, 1),
      supported_command_latency_ms_max: safeNumber(thresholds.supported_command_latency_ms_max, 2500),
      recovery_rto_minutes_max: safeNumber(thresholds.recovery_rto_minutes_max, 30),
      recovery_rpo_hours_max: safeNumber(thresholds.recovery_rpo_hours_max, 24),
      boundedness_regression_tolerance_pct: safeNumber(
        boundednessReleaseGate?.boundedness_budgets?.regression_tolerance_pct,
        Number.NaN,
      ),
    },
    thresholds: {
      ipc_success_rate: Number(ipcSuccessRate.toFixed(4)),
      receipt_completeness_rate: Number(receiptCompleteness.toFixed(4)),
      max_command_latency_ms: maxCommandLatencyMs,
      closure_command_latency_ms: closureCommandLatencyMs,
      observed_rto_minutes: observedRtoMinutes,
      observed_rpo_hours: observedRpoHours,
      node_critical_priority_one_missing_rust_count: nodeCriticalPriorityOneMissingRustCount,
      node_critical_migration_overdue_count: nodeCriticalMigrationOverdueCount,
      node_critical_ts_confinement_violation_count: nodeCriticalTsConfinementViolationCount,
      boundedness_failed_count: safeNumber(boundednessReleaseGate?.summary?.failed_count, Number.NaN),
      boundedness_warning_count: safeNumber(boundednessReleaseGate?.summary?.warning_count, Number.NaN),
    },
    boundedness_budgets: boundednessReleaseGate?.boundedness_budgets ?? {},
    boundedness_soak_projection: boundednessReleaseGate?.soak_projection ?? {},
    node_critical_path: {
      path: path.relative(root, nodeCriticalPath),
      ok: nodeCriticalInventoryOk,
      summary: nodeCriticalPathInventory?.summary ?? {},
      failed_ids: Array.isArray(nodeCriticalPathInventory?.failures)
        ? nodeCriticalPathInventory.failures.map((row: any) => cleanText(row?.id || '', 120))
        : [],
    },
    benchmark_classes: benchmarkClasses,
    benchmark_trends: {
      microkernel_shared_workload_delta_pct: Number.isFinite(microkernelSharedOps) &&
        Number.isFinite(safeNumber(benchmarkBaselineClasses?.microkernel_shared_workload, Number.NaN)) &&
        safeNumber(benchmarkBaselineClasses?.microkernel_shared_workload, Number.NaN) > 0
        ? Number((((microkernelSharedOps / safeNumber(benchmarkBaselineClasses?.microkernel_shared_workload, 1)) - 1) * 100).toFixed(3))
        : null,
      governed_command_path_delta_pct: Number.isFinite(governedCommandPathOps) &&
        Number.isFinite(safeNumber(benchmarkBaselineClasses?.governed_command_path, Number.NaN)) &&
        safeNumber(benchmarkBaselineClasses?.governed_command_path, Number.NaN) > 0
        ? Number((((governedCommandPathOps / safeNumber(benchmarkBaselineClasses?.governed_command_path, 1)) - 1) * 100).toFixed(3))
        : null,
      readiness_latency_delta_pct: Number.isFinite(readinessLatencyMs) &&
        Number.isFinite(safeNumber(benchmarkBaselineClasses?.readiness_latency_ms, Number.NaN)) &&
        safeNumber(benchmarkBaselineClasses?.readiness_latency_ms, Number.NaN) > 0
        ? Number((((readinessLatencyMs / safeNumber(benchmarkBaselineClasses?.readiness_latency_ms, 1)) - 1) * 100).toFixed(3))
        : null,
    },
    failed_gate_ids: gates.filter((row) => !row.ok).map((row) => row.id),
    gates,
  };

  return {
    root,
    outPath: resolveMaybe(root, normalizedArgs.outPath),
    report,
  };
}

export function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const result = buildReport(args);
  fs.mkdirSync(path.dirname(result.outPath), { recursive: true });
  fs.writeFileSync(result.outPath, `${JSON.stringify(result.report, null, 2)}\n`, 'utf8');
  process.stdout.write(`${JSON.stringify(result.report, null, 2)}\n`);
  return result.report.ok ? 0 : 1;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  buildReport,
  run,
};
