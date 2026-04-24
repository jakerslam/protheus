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
    releaseVerdictPath: 'core/local/artifacts/release_verdict_current.json',
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
    else if (token.startsWith('--release-verdict=')) out.releaseVerdictPath = cleanText(token.slice(18), 400);
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
  const releaseVerdictPath = resolveMaybe(root, normalizedArgs.releaseVerdictPath);
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
  const releaseVerdict = readJsonMaybe(releaseVerdictPath) ?? {};
  const boundednessReleaseGate = readJsonMaybe(boundednessReleaseGatePath) ?? {};
  const ipcSoak = readJsonFirst([ipcSoakPath, ipcSoakFallbackPath]) ?? {};
  const dr = readJsonMaybe(drPath) ?? {};
  const benchmark = readJsonMaybe(benchmarkPath) ?? {};
  const benchmarkBaseline = benchmarkBaselinePath ? readJsonMaybe(benchmarkBaselinePath) ?? {} : null;
  const baselineScorecard = baselinePath ? readJsonMaybe(baselinePath) ?? {} : null;
  const channel = releaseChannel(semver?.release_channel);
  const semverReleaseChannelRaw = cleanText(semver?.release_channel || '', 40).toLowerCase();
  const requireReleaseArtifacts = Boolean(normalizedArgs.requireReleaseArtifacts);
  const requireBaseline = Boolean(normalizedArgs.requireBaseline);
  const finalStage = normalizedArgs.stage === 'final';
  const releaseEvidenceFlow = closurePolicy?.release_evidence_flow ?? {};
  const requiredScorecardStage = cleanText(
    releaseEvidenceFlow?.scorecard_stage || 'prebundle',
    40,
  ).toLowerCase();
  const requiredFinalStage = cleanText(
    releaseEvidenceFlow?.final_closure_stage || 'final',
    40,
  ).toLowerCase();
  const releaseEvidenceFlowStageTokensCanonical =
    (requiredScorecardStage === 'prebundle' || requiredScorecardStage === 'final') &&
    (requiredFinalStage === 'prebundle' || requiredFinalStage === 'final');
  const releaseEvidenceFlowStageTokensDistinct = requiredScorecardStage !== requiredFinalStage;
  const invokedStageAllowedByPolicy =
    normalizedArgs.stage === requiredScorecardStage || normalizedArgs.stage === requiredFinalStage;
  const invokedStageMatchesPolicyRole = finalStage
    ? normalizedArgs.stage === requiredFinalStage
    : normalizedArgs.stage === requiredScorecardStage;

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
  const nodeCriticalFailureIdTokenRegex = /^[a-z0-9:_-]+$/;
  const nodeCriticalFailedIds = Array.isArray(nodeCriticalPathInventory?.failures)
    ? nodeCriticalPathInventory.failures
        .map((row: any) => cleanText(row?.id || '', 120))
        .filter(Boolean)
    : [];
  const nodeCriticalFailedIdsUnique = nodeCriticalFailedIds.length === new Set(nodeCriticalFailedIds).size;
  const nodeCriticalFailedIdsTokenValid = nodeCriticalFailedIds.every((id) =>
    nodeCriticalFailureIdTokenRegex.test(id),
  );
  const nodeCriticalFailedIdsSummaryCount = safeNumber(
    nodeCriticalPathInventory?.summary?.failed_count,
    Number.NaN,
  );
  const nodeCriticalFailedIdsCountParity =
    !Number.isFinite(nodeCriticalFailedIdsSummaryCount) ||
    nodeCriticalFailedIds.length === nodeCriticalFailedIdsSummaryCount;
  const nodeCriticalSummaryScalarContract =
    Number.isInteger(nodeCriticalPriorityOneMissingRustCount) &&
    nodeCriticalPriorityOneMissingRustCount >= 0 &&
    Number.isInteger(nodeCriticalMigrationOverdueCount) &&
    nodeCriticalMigrationOverdueCount >= 0 &&
    Number.isInteger(nodeCriticalTsConfinementViolationCount) &&
    nodeCriticalTsConfinementViolationCount >= 0;
  const closureStage = cleanText(closure?.stage || '', 40).toLowerCase();
  const closureSummaryInvokedStage = cleanText(
    closure?.summary?.invoked_stage || closure?.summary?.requested_stage || '',
    40,
  ).toLowerCase();
  const closureStageCanonical =
    closureStage.length === 0 || closureStage === 'prebundle' || closureStage === 'final';
  const closureSummaryStageCanonical =
    closureSummaryInvokedStage.length === 0 ||
    closureSummaryInvokedStage === 'prebundle' ||
    closureSummaryInvokedStage === 'final';
  const closureStageMatchesInvocation =
    closureStage.length === 0 || closureStage === normalizedArgs.stage;
  const closureSummaryStageMatchesInvocation =
    closureSummaryInvokedStage.length === 0 ||
    closureSummaryInvokedStage === normalizedArgs.stage;
  const boundednessFailedCount = safeNumber(
    boundednessReleaseGate?.summary?.failed_count,
    Number.NaN,
  );
  const boundednessWarningCount = safeNumber(
    boundednessReleaseGate?.summary?.warning_count,
    Number.NaN,
  );
  const boundednessSummaryScalarContract =
    Number.isInteger(boundednessFailedCount) &&
    boundednessFailedCount >= 0 &&
    Number.isInteger(boundednessWarningCount) &&
    boundednessWarningCount >= 0;
  const boundednessFailedCountScalarContract =
    Number.isInteger(boundednessFailedCount) && boundednessFailedCount >= 0;
  const boundednessWarningCountScalarContract =
    Number.isInteger(boundednessWarningCount) && boundednessWarningCount >= 0;
  const supportBundleMetricScalarContract =
    Number.isFinite(receiptCompleteness) &&
    Number.isFinite(maxCommandLatencyMs) &&
    Number.isFinite(closureCommandLatencyMs) &&
    Number.isFinite(observedRtoMinutes) &&
    Number.isFinite(observedRpoHours);
  const ipcSuccessRateScalarRangeContract =
    Number.isFinite(ipcSuccessRate) &&
    ipcSuccessRate >= 0 &&
    ipcSuccessRate <= 1;
  const receiptCompletenessScalarRangeContract =
    Number.isFinite(receiptCompleteness) &&
    receiptCompleteness >= 0 &&
    receiptCompleteness <= 1;
  const maxCommandLatencyScalarContract =
    Number.isFinite(maxCommandLatencyMs) && maxCommandLatencyMs >= 0;
  const closureCommandLatencyScalarContract =
    Number.isFinite(closureCommandLatencyMs) && closureCommandLatencyMs >= 0;
  const observedRtoScalarContract =
    Number.isFinite(observedRtoMinutes) && observedRtoMinutes >= 0;
  const observedRpoScalarContract =
    Number.isFinite(observedRpoHours) && observedRpoHours >= 0;
  const releaseVerdictPresent =
    Boolean(releaseVerdict && typeof releaseVerdict === 'object') &&
    Object.keys(releaseVerdict).length > 0;
  const releaseVerdictStage = cleanText(releaseVerdict?.stage || '', 40).toLowerCase();
  const releaseVerdictRequestedStage = cleanText(
    releaseVerdict?.summary?.requested_stage || '',
    40,
  ).toLowerCase();
  const releaseVerdictRcStage = cleanText(
    releaseVerdict?.summary?.rc_stage || '',
    40,
  ).toLowerCase();
  const releaseVerdictRcStageMode = cleanText(
    releaseVerdict?.summary?.rc_stage_mode || '',
    40,
  ).toLowerCase();
  const releaseVerdictStageTokensCanonical =
    [releaseVerdictStage, releaseVerdictRequestedStage, releaseVerdictRcStage].every(
      (token) =>
        token.length === 0 || token === 'prebundle' || token === 'final',
    );
  const releaseVerdictRcStageModeCanonical =
    releaseVerdictRcStageMode.length === 0 ||
    releaseVerdictRcStageMode === 'strict_final' ||
    releaseVerdictRcStageMode === 'prebundle_mixed';
  const releaseVerdictStageMatchesInvocation =
    releaseVerdictStage.length > 0 &&
    releaseVerdictStage === normalizedArgs.stage &&
    (releaseVerdictRequestedStage.length === 0 ||
      releaseVerdictRequestedStage === normalizedArgs.stage);
  const releaseVerdictRcStageModeMatchesRcStage =
    releaseVerdictRcStage.length === 0 ||
    releaseVerdictRcStageMode.length === 0 ||
    (releaseVerdictRcStage === 'final' && releaseVerdictRcStageMode === 'strict_final') ||
    (releaseVerdictRcStage === 'prebundle' && releaseVerdictRcStageMode === 'prebundle_mixed');
  const releaseVerdictBlockingFailedCount = safeNumber(
    releaseVerdict?.summary?.rc_blocking_failed_count,
    Number.NaN,
  );
  const releaseVerdictNonBlockingFailedCount = safeNumber(
    releaseVerdict?.summary?.rc_non_blocking_failed_count,
    Number.NaN,
  );
  const releaseVerdictFailedCount = safeNumber(
    releaseVerdict?.summary?.failed_count,
    Number.NaN,
  );
  const releaseVerdictFailedGateIds = Array.isArray(releaseVerdict?.failed_gate_ids)
    ? releaseVerdict.failed_gate_ids.map((row: any) => cleanText(row, 160)).filter(Boolean)
    : [];
  const releaseVerdictFailedGateIdTokenRegex = /^[a-z0-9:_-]+$/;
  const releaseVerdictFailedGateIdSet = new Set(releaseVerdictFailedGateIds);
  const releaseVerdictFailedGateIdsUnique =
    releaseVerdictFailedGateIds.length === releaseVerdictFailedGateIdSet.size;
  const releaseVerdictFailedGateIdsTokenValid = releaseVerdictFailedGateIds.every((id) =>
    releaseVerdictFailedGateIdTokenRegex.test(id),
  );
  const releaseVerdictSummaryScalarContract =
    Number.isInteger(releaseVerdictBlockingFailedCount) &&
    releaseVerdictBlockingFailedCount >= 0 &&
    Number.isInteger(releaseVerdictNonBlockingFailedCount) &&
    releaseVerdictNonBlockingFailedCount >= 0 &&
    Number.isInteger(releaseVerdictFailedCount) &&
    releaseVerdictFailedCount >= 0;
  const releaseVerdictFailedGateIdsCountParity =
    Number.isInteger(releaseVerdictFailedCount) &&
    releaseVerdictFailedCount >= 0 &&
    releaseVerdictFailedGateIds.length === releaseVerdictFailedCount;
  const releaseVerdictFailedGateIdsNonEmptyOnFailure =
    !Number.isInteger(releaseVerdictFailedCount) ||
    releaseVerdictFailedCount <= 0 ||
    releaseVerdictFailedGateIds.length > 0;
  const releaseVerdictFailedGateIdsEmptyOnSuccess =
    !Number.isInteger(releaseVerdictFailedCount) ||
    releaseVerdictFailedCount > 0 ||
    releaseVerdictFailedGateIds.length === 0;
  const releaseVerdictBlockingCountLeqFailed =
    !Number.isInteger(releaseVerdictBlockingFailedCount) ||
    !Number.isInteger(releaseVerdictFailedCount) ||
    releaseVerdictBlockingFailedCount <= releaseVerdictFailedCount;
  const releaseVerdictNonBlockingCountLeqFailed =
    !Number.isInteger(releaseVerdictNonBlockingFailedCount) ||
    !Number.isInteger(releaseVerdictFailedCount) ||
    releaseVerdictNonBlockingFailedCount <= releaseVerdictFailedCount;
  const releaseVerdictPresentRequiresStageTokens =
    !releaseVerdictPresent ||
    (releaseVerdictStage.length > 0 &&
      releaseVerdictRequestedStage.length > 0 &&
      releaseVerdictRcStage.length > 0);
  const releaseVerdictRcStageMatchesStage =
    !releaseVerdictPresent ||
    releaseVerdictRcStage.length === 0 ||
    releaseVerdictStage.length === 0 ||
    releaseVerdictRcStage === releaseVerdictStage;
  const releaseVerdictFailurePartitionContract =
    Number.isInteger(releaseVerdictBlockingFailedCount) &&
    releaseVerdictBlockingFailedCount >= 0 &&
    Number.isInteger(releaseVerdictNonBlockingFailedCount) &&
    releaseVerdictNonBlockingFailedCount >= 0 &&
    Number.isInteger(releaseVerdictFailedCount) &&
    releaseVerdictFailedCount >= 0 &&
    releaseVerdictBlockingFailedCount + releaseVerdictNonBlockingFailedCount ===
      releaseVerdictFailedCount &&
    releaseVerdictFailedGateIds.length === releaseVerdictFailedCount;
  const releaseVerdictFailedCountScalarContract =
    Number.isInteger(releaseVerdictFailedCount) && releaseVerdictFailedCount >= 0;
  const releaseVerdictBlockingNonBlockingScalarContract =
    Number.isInteger(releaseVerdictBlockingFailedCount) &&
    releaseVerdictBlockingFailedCount >= 0 &&
    Number.isInteger(releaseVerdictNonBlockingFailedCount) &&
    releaseVerdictNonBlockingFailedCount >= 0;
  const releaseVerdictPresentWhenFinalStageContract =
    !finalStage || releaseVerdictPresent;
  const releaseEvidenceFlowSupportBundlePrecedesBoolean =
    typeof releaseEvidenceFlow?.support_bundle_precedes_final_closure === 'boolean';
  const closureSummaryPassBooleanContract =
    closure?.summary?.pass == null || typeof closure?.summary?.pass === 'boolean';
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
  const benchmarkClassDirectionTokenContract = Object.values(benchmarkClasses).every((row: any) => {
    const direction = cleanText(row?.direction || '', 80);
    return direction === 'higher_is_better' || direction === 'lower_is_better';
  });
  const benchmarkClassMetricTokenContract = Object.values(benchmarkClasses).every((row: any) =>
    /^[a-z0-9_]+$/.test(cleanText(row?.metric || '', 120)),
  );
  const benchmarkClassSourcePathTokenContract = Object.values(benchmarkClasses).every((row: any) =>
    /^[A-Za-z0-9._/-]+$/.test(cleanText(row?.source_path || '', 260)),
  );
  const benchmarkClassValueScalarContract = Object.values(benchmarkClasses).every((row: any) => {
    const value = row?.value;
    return value == null || Number.isFinite(Number(value));
  });
  const benchmarkClassReadinessLatencyScalarContract = Number.isFinite(readinessLatencyMs);
  const policyThresholdIpcMin = safeNumber(thresholds.ipc_success_rate_min, 0.95);
  const policyThresholdReceiptMin = safeNumber(thresholds.receipt_completeness_rate_min, 1);
  const policyThresholdLatencyMax = safeNumber(thresholds.supported_command_latency_ms_max, 2500);
  const policyThresholdRtoMax = safeNumber(thresholds.recovery_rto_minutes_max, 30);
  const policyThresholdRpoMax = safeNumber(thresholds.recovery_rpo_hours_max, 24);
  const policyThresholdScalarContract =
    Number.isFinite(policyThresholdIpcMin) &&
    policyThresholdIpcMin >= 0 &&
    policyThresholdIpcMin <= 1 &&
    Number.isFinite(policyThresholdReceiptMin) &&
    policyThresholdReceiptMin >= 0 &&
    policyThresholdReceiptMin <= 1 &&
    Number.isFinite(policyThresholdLatencyMax) &&
    policyThresholdLatencyMax >= 0 &&
    Number.isFinite(policyThresholdRtoMax) &&
    policyThresholdRtoMax >= 0 &&
    Number.isFinite(policyThresholdRpoMax) &&
    policyThresholdRpoMax >= 0;
  const stageTokenContract =
    normalizedArgs.stage === 'prebundle' || normalizedArgs.stage === 'final';
  const semverReleaseChannelTokenContract =
    semverReleaseChannelRaw === '' ||
    semverReleaseChannelRaw === 'alpha' ||
    semverReleaseChannelRaw === 'beta' ||
    semverReleaseChannelRaw === 'stable';
  const canaryRequirementChannelConsistencyContract =
    canaryRequired === (requireReleaseArtifacts && channel === 'stable');
  const releaseArtifactsRequireTagContract =
    !requireReleaseArtifacts || cleanText(semver?.next_tag || '', 120).length > 0;
  const finalStageSupportBundlePrecedesClosureContract =
    !finalStage || releaseEvidenceFlow?.support_bundle_precedes_final_closure !== false;
  const benchmarkRegressionBudgetScalarContract =
    Number.isFinite(safeNumber(benchmarkBudgets?.microkernel_shared_workload_pct_max, Number.NaN)) &&
    safeNumber(benchmarkBudgets?.microkernel_shared_workload_pct_max, Number.NaN) >= 0 &&
    safeNumber(benchmarkBudgets?.microkernel_shared_workload_pct_max, Number.NaN) <= 1 &&
    Number.isFinite(safeNumber(benchmarkBudgets?.governed_command_path_pct_max, Number.NaN)) &&
    safeNumber(benchmarkBudgets?.governed_command_path_pct_max, Number.NaN) >= 0 &&
    safeNumber(benchmarkBudgets?.governed_command_path_pct_max, Number.NaN) <= 1 &&
    Number.isFinite(safeNumber(benchmarkBudgets?.readiness_latency_pct_max, Number.NaN)) &&
    safeNumber(benchmarkBudgets?.readiness_latency_pct_max, Number.NaN) >= 0 &&
    safeNumber(benchmarkBudgets?.readiness_latency_pct_max, Number.NaN) <= 1;
  const benchmarkBaselinePathRequiredContract =
    !requireBaseline || cleanText(normalizedArgs.baselinePath || '', 260).length > 0;

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
  const benchmarkTrendDeltaScalarContract = (() => {
    const microDelta =
      Number.isFinite(microkernelSharedOps) &&
      Number.isFinite(safeNumber(benchmarkBaselineClasses?.microkernel_shared_workload, Number.NaN)) &&
      safeNumber(benchmarkBaselineClasses?.microkernel_shared_workload, Number.NaN) > 0
        ? Number((((microkernelSharedOps / safeNumber(benchmarkBaselineClasses?.microkernel_shared_workload, 1)) - 1) * 100).toFixed(3))
        : null;
    const governedDelta =
      Number.isFinite(governedCommandPathOps) &&
      Number.isFinite(safeNumber(benchmarkBaselineClasses?.governed_command_path, Number.NaN)) &&
      safeNumber(benchmarkBaselineClasses?.governed_command_path, Number.NaN) > 0
        ? Number((((governedCommandPathOps / safeNumber(benchmarkBaselineClasses?.governed_command_path, 1)) - 1) * 100).toFixed(3))
        : null;
    const readinessDelta =
      Number.isFinite(readinessLatencyMs) &&
      Number.isFinite(safeNumber(benchmarkBaselineClasses?.readiness_latency_ms, Number.NaN)) &&
      safeNumber(benchmarkBaselineClasses?.readiness_latency_ms, Number.NaN) > 0
        ? Number((((readinessLatencyMs / safeNumber(benchmarkBaselineClasses?.readiness_latency_ms, 1)) - 1) * 100).toFixed(3))
        : null;
    return [microDelta, governedDelta, readinessDelta].every(
      (value) => value == null || Number.isFinite(Number(value)),
    );
  })();
  const supportBundleIncidentTruthPackageShapeContract = (() => {
    const pkg = supportBundle?.incident_truth_package;
    if (!pkg || typeof pkg !== 'object') return !finalStage;
    const failedChecks = pkg.failed_checks;
    if (!Array.isArray(failedChecks)) return false;
    return failedChecks.every(
      (row: any) => typeof row === 'string' && cleanText(row, 160).length > 0,
    );
  })();

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
    gateRow(
      'release_scorecard_stage_token_contract_v3',
      stageTokenContract,
      `stage=${normalizedArgs.stage}`,
    ),
    gateRow(
      'release_channel_token_contract_v3',
      semverReleaseChannelTokenContract,
      `raw_channel=${semverReleaseChannelRaw || 'missing'};resolved_channel=${channel}`,
    ),
    gateRow(
      'release_evidence_flow_stage_tokens_canonical',
      releaseEvidenceFlowStageTokensCanonical,
      `scorecard_stage=${requiredScorecardStage || 'missing'};final_stage=${requiredFinalStage || 'missing'}`
    ),
    gateRow(
      'release_evidence_flow_stage_tokens_distinct',
      releaseEvidenceFlowStageTokensDistinct,
      `scorecard_stage=${requiredScorecardStage || 'missing'};final_stage=${requiredFinalStage || 'missing'}`
    ),
    gateRow(
      'release_evidence_flow_invoked_stage_allowed',
      invokedStageAllowedByPolicy,
      `invoked_stage=${normalizedArgs.stage};scorecard_stage=${requiredScorecardStage || 'missing'};final_stage=${requiredFinalStage || 'missing'}`
    ),
    gateRow(
      'release_evidence_flow_invoked_stage_matches_role',
      invokedStageMatchesPolicyRole,
      `invoked_stage=${normalizedArgs.stage};expected_stage=${finalStage ? requiredFinalStage || 'missing' : requiredScorecardStage || 'missing'}`
    ),
    gateRow(
      'canary_requirement_channel_consistency_contract_v3',
      canaryRequirementChannelConsistencyContract,
      `require_release_artifacts=${requireReleaseArtifacts};channel=${channel};canary_required=${canaryRequired}`,
    ),
    gateRow(
      'release_artifacts_require_next_tag_contract_v3',
      releaseArtifactsRequireTagContract,
      `require_release_artifacts=${requireReleaseArtifacts};next_tag=${cleanText(semver?.next_tag ?? 'none', 120)}`,
    ),
    gateRow(
      'final_stage_support_bundle_precedes_closure_contract_v3',
      finalStageSupportBundlePrecedesClosureContract,
      `final_stage=${finalStage};support_bundle_precedes=${String(releaseEvidenceFlow?.support_bundle_precedes_final_closure)}`,
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
    optionalGateRow(
      'stateful_upgrade_rollback_gate',
      finalStage,
      stateCompat?.ok === true,
      `errors=${Array.isArray(stateCompat?.errors) ? stateCompat.errors.length : 0}`
    ),
    optionalGateRow(
      'live_upgrade_rollback_rehearsal',
      finalStage,
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
      'support_bundle_metrics_scalar_contract',
      supportBundleMetricScalarContract,
      `receipt=${Number.isFinite(receiptCompleteness)};max_latency=${Number.isFinite(maxCommandLatencyMs)};closure_latency=${Number.isFinite(closureCommandLatencyMs)};rto=${Number.isFinite(observedRtoMinutes)};rpo=${Number.isFinite(observedRpoHours)}`
    ),
    gateRow(
      'ipc_success_rate_scalar_range_contract_v3',
      ipcSuccessRateScalarRangeContract,
      `value=${ipcSuccessRate.toFixed(4)}`,
    ),
    gateRow(
      'receipt_completeness_scalar_range_contract_v3',
      receiptCompletenessScalarRangeContract,
      `value=${receiptCompleteness.toFixed(4)}`,
    ),
    gateRow(
      'max_command_latency_scalar_contract_v3',
      maxCommandLatencyScalarContract,
      `value=${maxCommandLatencyMs}`,
    ),
    gateRow(
      'closure_command_latency_scalar_contract_v3',
      closureCommandLatencyScalarContract,
      `value=${closureCommandLatencyMs}`,
    ),
    gateRow(
      'observed_rto_scalar_contract_v3',
      observedRtoScalarContract,
      `value=${observedRtoMinutes}`,
    ),
    gateRow(
      'observed_rpo_scalar_contract_v3',
      observedRpoScalarContract,
      `value=${observedRpoHours}`,
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
    optionalGateRow(
      'boundedness_release_gate',
      finalStage,
      boundednessReleaseGate?.ok === true,
      `failed=${boundednessFailedCount};warnings=${boundednessWarningCount}`
    ),
    optionalGateRow(
      'boundedness_release_gate_summary_scalar_contract',
      finalStage,
      boundednessSummaryScalarContract,
      `failed=${boundednessFailedCount};warnings=${boundednessWarningCount}`
    ),
    optionalGateRow(
      'boundedness_failed_count_scalar_contract_v3',
      finalStage,
      boundednessFailedCountScalarContract,
      `failed=${boundednessFailedCount}`,
    ),
    optionalGateRow(
      'boundedness_warning_count_scalar_contract_v3',
      finalStage,
      boundednessWarningCountScalarContract,
      `warnings=${boundednessWarningCount}`,
    ),
    optionalGateRow(
      'node_critical_path_inventory_gate',
      finalStage,
      nodeCriticalInventoryOk,
      `path=${path.relative(root, nodeCriticalPath)};ok=${nodeCriticalInventoryOk}`,
    ),
    optionalGateRow(
      'node_critical_priority_one_rust_target_coverage',
      finalStage,
      nodeCriticalPriorityOneMissingRustCount === 0,
      `value=${nodeCriticalPriorityOneMissingRustCount};expected=0`,
    ),
    optionalGateRow(
      'node_critical_migration_overdue_zero',
      finalStage,
      nodeCriticalMigrationOverdueCount === 0,
      `value=${nodeCriticalMigrationOverdueCount};expected=0`,
    ),
    optionalGateRow(
      'node_critical_ts_confinement_violations_zero',
      finalStage,
      nodeCriticalTsConfinementViolationCount === 0,
      `value=${nodeCriticalTsConfinementViolationCount};expected=0`,
    ),
    optionalGateRow(
      'node_critical_summary_scalar_contract',
      finalStage,
      nodeCriticalSummaryScalarContract,
      `priority_one_missing_rust=${nodeCriticalPriorityOneMissingRustCount};migration_overdue=${nodeCriticalMigrationOverdueCount};ts_confinement_violations=${nodeCriticalTsConfinementViolationCount}`,
    ),
    optionalGateRow(
      'production_closure_stage_tokens_canonical',
      finalStage,
      closureStageCanonical && closureSummaryStageCanonical,
      `closure_stage=${closureStage || 'missing'};closure_summary_stage=${closureSummaryInvokedStage || 'missing'}`,
    ),
    optionalGateRow(
      'production_closure_stage_matches_invocation',
      finalStage,
      closureStageMatchesInvocation && closureSummaryStageMatchesInvocation,
      `invoked_stage=${normalizedArgs.stage};closure_stage=${closureStage || 'missing'};closure_summary_stage=${closureSummaryInvokedStage || 'missing'}`,
    ),
    optionalGateRow(
      'release_verdict_stage_tokens_canonical',
      releaseVerdictPresent,
      releaseVerdictStageTokensCanonical && releaseVerdictRcStageModeCanonical,
      `stage=${releaseVerdictStage || 'missing'};requested_stage=${releaseVerdictRequestedStage || 'missing'};rc_stage=${releaseVerdictRcStage || 'missing'};rc_stage_mode=${releaseVerdictRcStageMode || 'missing'}`
    ),
    optionalGateRow(
      'release_verdict_stage_matches_invocation',
      releaseVerdictPresent,
      releaseVerdictStageMatchesInvocation && releaseVerdictRcStageModeMatchesRcStage,
      `invoked_stage=${normalizedArgs.stage};stage=${releaseVerdictStage || 'missing'};requested_stage=${releaseVerdictRequestedStage || 'missing'};rc_stage=${releaseVerdictRcStage || 'missing'};rc_stage_mode=${releaseVerdictRcStageMode || 'missing'}`
    ),
    optionalGateRow(
      'release_verdict_failure_partition_contract',
      releaseVerdictPresent,
      releaseVerdictFailurePartitionContract,
      `blocking_failed=${releaseVerdictBlockingFailedCount};non_blocking_failed=${releaseVerdictNonBlockingFailedCount};failed=${releaseVerdictFailedCount};failed_gate_ids=${releaseVerdictFailedGateIds.length}`,
    ),
    optionalGateRow(
      'release_verdict_failed_gate_ids_unique',
      releaseVerdictPresent,
      releaseVerdictFailedGateIdsUnique,
      `failed_gate_ids=${releaseVerdictFailedGateIds.length};unique_failed_gate_ids=${releaseVerdictFailedGateIdSet.size}`,
    ),
    optionalGateRow(
      'release_verdict_failed_gate_ids_token_contract_v2',
      releaseVerdictPresent,
      releaseVerdictFailedGateIdsTokenValid,
      `failed_gate_ids=${releaseVerdictFailedGateIds.length}`,
    ),
    optionalGateRow(
      'release_verdict_failed_gate_ids_count_parity_contract_v2',
      releaseVerdictPresent,
      releaseVerdictFailedGateIdsCountParity,
      `failed_gate_ids=${releaseVerdictFailedGateIds.length};failed_count=${releaseVerdictFailedCount}`,
    ),
    optionalGateRow(
      'release_verdict_failed_gate_ids_nonempty_on_failure_contract_v2',
      releaseVerdictPresent,
      releaseVerdictFailedGateIdsNonEmptyOnFailure,
      `failed_gate_ids=${releaseVerdictFailedGateIds.length};failed_count=${releaseVerdictFailedCount}`,
    ),
    optionalGateRow(
      'release_verdict_failed_gate_ids_empty_on_success_contract_v2',
      releaseVerdictPresent,
      releaseVerdictFailedGateIdsEmptyOnSuccess,
      `failed_gate_ids=${releaseVerdictFailedGateIds.length};failed_count=${releaseVerdictFailedCount}`,
    ),
    optionalGateRow(
      'release_verdict_summary_scalar_contract_v2',
      releaseVerdictPresent,
      releaseVerdictSummaryScalarContract,
      `blocking=${releaseVerdictBlockingFailedCount};non_blocking=${releaseVerdictNonBlockingFailedCount};failed=${releaseVerdictFailedCount}`,
    ),
    optionalGateRow(
      'release_verdict_blocking_count_leq_failed_contract_v2',
      releaseVerdictPresent,
      releaseVerdictBlockingCountLeqFailed,
      `blocking=${releaseVerdictBlockingFailedCount};failed=${releaseVerdictFailedCount}`,
    ),
    optionalGateRow(
      'release_verdict_non_blocking_count_leq_failed_contract_v2',
      releaseVerdictPresent,
      releaseVerdictNonBlockingCountLeqFailed,
      `non_blocking=${releaseVerdictNonBlockingFailedCount};failed=${releaseVerdictFailedCount}`,
    ),
    optionalGateRow(
      'release_verdict_failed_count_scalar_contract_v3',
      releaseVerdictPresent,
      releaseVerdictFailedCountScalarContract,
      `failed=${releaseVerdictFailedCount}`,
    ),
    optionalGateRow(
      'release_verdict_blocking_non_blocking_scalar_contract_v3',
      releaseVerdictPresent,
      releaseVerdictBlockingNonBlockingScalarContract,
      `blocking=${releaseVerdictBlockingFailedCount};non_blocking=${releaseVerdictNonBlockingFailedCount}`,
    ),
    optionalGateRow(
      'release_verdict_present_when_final_stage_contract_v3',
      finalStage,
      releaseVerdictPresentWhenFinalStageContract,
      `final_stage=${finalStage};release_verdict_present=${releaseVerdictPresent}`,
    ),
    optionalGateRow(
      'release_verdict_present_requires_stage_tokens_contract_v2',
      releaseVerdictPresent,
      releaseVerdictPresentRequiresStageTokens,
      `stage=${releaseVerdictStage || 'missing'};requested_stage=${releaseVerdictRequestedStage || 'missing'};rc_stage=${releaseVerdictRcStage || 'missing'}`,
    ),
    optionalGateRow(
      'release_verdict_rc_stage_matches_stage_contract_v2',
      releaseVerdictPresent,
      releaseVerdictRcStageMatchesStage,
      `stage=${releaseVerdictStage || 'missing'};rc_stage=${releaseVerdictRcStage || 'missing'}`,
    ),
    gateRow(
      'release_evidence_flow_support_bundle_precedes_boolean_contract_v2',
      releaseEvidenceFlowSupportBundlePrecedesBoolean,
      `value_type=${typeof releaseEvidenceFlow?.support_bundle_precedes_final_closure}`,
    ),
    optionalGateRow(
      'production_closure_summary_pass_boolean_contract_v2',
      finalStage,
      closureSummaryPassBooleanContract,
      `summary_pass_type=${typeof closure?.summary?.pass}`,
    ),
    optionalGateRow(
      'node_critical_failed_ids_token_contract_v2',
      finalStage,
      nodeCriticalFailedIdsTokenValid,
      `failed_ids=${nodeCriticalFailedIds.length}`,
    ),
    optionalGateRow(
      'node_critical_failed_ids_unique_contract_v2',
      finalStage,
      nodeCriticalFailedIdsUnique,
      `failed_ids=${nodeCriticalFailedIds.length};unique_failed_ids=${new Set(nodeCriticalFailedIds).size}`,
    ),
    optionalGateRow(
      'node_critical_failed_ids_count_parity_contract_v2',
      finalStage,
      nodeCriticalFailedIdsCountParity,
      `failed_ids=${nodeCriticalFailedIds.length};summary_failed_count=${Number.isFinite(nodeCriticalFailedIdsSummaryCount) ? nodeCriticalFailedIdsSummaryCount : 'missing'}`,
    ),
    gateRow(
      'benchmark_class_direction_token_contract_v2',
      benchmarkClassDirectionTokenContract,
      `directions=${Object.values(benchmarkClasses).map((row: any) => cleanText(row?.direction || 'missing', 60)).join(',')}`,
    ),
    gateRow(
      'benchmark_class_metric_token_contract_v2',
      benchmarkClassMetricTokenContract,
      `metrics=${Object.values(benchmarkClasses).map((row: any) => cleanText(row?.metric || 'missing', 80)).join(',')}`,
    ),
    gateRow(
      'benchmark_class_source_path_token_contract_v2',
      benchmarkClassSourcePathTokenContract,
      `source_paths=${Object.values(benchmarkClasses).map((row: any) => cleanText(row?.source_path || 'missing', 120)).join(',')}`,
    ),
    gateRow(
      'benchmark_class_value_scalar_contract_v2',
      benchmarkClassValueScalarContract,
      `values=${Object.values(benchmarkClasses).map((row: any) => String(row?.value)).join(',')}`,
    ),
    gateRow(
      'benchmark_class_readiness_latency_scalar_contract_v2',
      benchmarkClassReadinessLatencyScalarContract,
      `readiness_latency_ms=${Number.isFinite(readinessLatencyMs) ? readinessLatencyMs : 'missing'}`,
    ),
    gateRow(
      'policy_threshold_scalar_contract_v2',
      policyThresholdScalarContract,
      `ipc_min=${policyThresholdIpcMin};receipt_min=${policyThresholdReceiptMin};latency_max=${policyThresholdLatencyMax};rto_max=${policyThresholdRtoMax};rpo_max=${policyThresholdRpoMax}`,
    ),
    gateRow(
      'benchmark_regression_budget_scalar_contract_v3',
      benchmarkRegressionBudgetScalarContract,
      `microkernel=${safeNumber(benchmarkBudgets?.microkernel_shared_workload_pct_max, Number.NaN)};governed=${safeNumber(benchmarkBudgets?.governed_command_path_pct_max, Number.NaN)};readiness=${safeNumber(benchmarkBudgets?.readiness_latency_pct_max, Number.NaN)}`,
    ),
    optionalGateRow(
      'benchmark_baseline_path_required_contract_v3',
      requireBaseline,
      benchmarkBaselinePathRequiredContract,
      `require_baseline=${requireBaseline};baseline_path=${cleanText(normalizedArgs.baselinePath || '', 260) || 'missing'}`,
    ),
    gateRow(
      'benchmark_trend_delta_scalar_contract_v3',
      benchmarkTrendDeltaScalarContract,
      `baseline_available=${baselineAvailable};benchmark_baseline_available=${Boolean(benchmarkBaseline && typeof benchmarkBaseline === 'object')}`,
    ),
    gateRow(
      'support_bundle_incident_truth_package_shape_contract_v3',
      supportBundleIncidentTruthPackageShapeContract,
      `final_stage=${finalStage};has_incident_truth_package=${Boolean(supportBundle?.incident_truth_package)}`,
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
    release_evidence_flow: {
      scorecard_stage: requiredScorecardStage,
      final_stage: requiredFinalStage,
      stage_tokens_canonical: releaseEvidenceFlowStageTokensCanonical,
      stage_tokens_distinct: releaseEvidenceFlowStageTokensDistinct,
      invoked_stage_allowed: invokedStageAllowedByPolicy,
      invoked_stage_matches_role: invokedStageMatchesPolicyRole,
      support_bundle_precedes_final_closure:
        releaseEvidenceFlow?.support_bundle_precedes_final_closure !== false,
    },
    closure_contract: {
      closure_stage: closureStage,
      closure_summary_stage: closureSummaryInvokedStage,
      closure_stage_canonical: closureStageCanonical,
      closure_summary_stage_canonical: closureSummaryStageCanonical,
      closure_stage_matches_invocation: closureStageMatchesInvocation,
      closure_summary_stage_matches_invocation: closureSummaryStageMatchesInvocation,
    },
    release_verdict: {
      path: path.relative(root, releaseVerdictPath),
      observed: releaseVerdictPresent,
      stage: releaseVerdictStage,
      requested_stage: releaseVerdictRequestedStage,
      rc_stage: releaseVerdictRcStage,
      rc_stage_mode: releaseVerdictRcStageMode,
      stage_tokens_canonical: releaseVerdictStageTokensCanonical,
      rc_stage_mode_canonical: releaseVerdictRcStageModeCanonical,
      stage_matches_invocation: releaseVerdictStageMatchesInvocation,
      rc_stage_mode_matches_rc_stage: releaseVerdictRcStageModeMatchesRcStage,
      failed_count: releaseVerdictFailedCount,
      blocking_failed_count: releaseVerdictBlockingFailedCount,
      non_blocking_failed_count: releaseVerdictNonBlockingFailedCount,
      failed_gate_ids: releaseVerdictFailedGateIds,
      failed_gate_ids_unique: releaseVerdictFailedGateIdsUnique,
      failure_partition_contract: releaseVerdictFailurePartitionContract,
    },
    data_contracts: {
      support_bundle_metrics_scalar_contract: supportBundleMetricScalarContract,
      boundedness_summary_scalar_contract: boundednessSummaryScalarContract,
      node_critical_summary_scalar_contract: nodeCriticalSummaryScalarContract,
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
      boundedness_failed_count: boundednessFailedCount,
      boundedness_warning_count: boundednessWarningCount,
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
