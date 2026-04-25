#!/usr/bin/env tsx

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/runtime_boundedness_release_gate_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    boundednessEvidencePath: cleanText(
      readFlag(argv, 'boundedness-evidence') ||
        'core/local/artifacts/runtime_boundedness_72h_evidence_current.json',
      400,
    ),
    boundednessProfilesPath: cleanText(
      readFlag(argv, 'boundedness-profiles') ||
        'core/local/artifacts/runtime_boundedness_profiles_current.json',
      400,
    ),
    queueBackpressureGatePath: cleanText(
      readFlag(argv, 'queue-backpressure-gate') ||
        'core/local/artifacts/queue_backpressure_policy_gate_current.json',
      400,
    ),
    dashboardSurfaceGuardPath: cleanText(
      readFlag(argv, 'dashboard-surface-guard') ||
        'core/local/artifacts/dashboard_surface_authority_guard_current.json',
      400,
    ),
    layer2ReplayPath: cleanText(
      readFlag(argv, 'layer2-replay') ||
        'core/local/artifacts/layer2_receipt_replay_current.json',
      400,
    ),
    multiDaySoakPath: cleanText(
      readFlag(argv, 'multi-day-soak') ||
        'core/local/artifacts/runtime_multi_day_soak_evidence_current.json',
      400,
    ),
    harnessRootPath: cleanText(
      readFlag(argv, 'harness-root') || 'core/local/artifacts',
      400,
    ),
    boundednessReportTemplate: cleanText(
      readFlag(argv, 'boundedness-report-template') ||
        'core/local/artifacts/runtime_boundedness_report_{profile}_current.json',
      400,
    ),
    boundednessBaselineTemplate: cleanText(
      readFlag(argv, 'boundedness-baseline-template') ||
        'core/local/artifacts/runtime_boundedness_report_{profile}_baseline.json',
      400,
    ),
    gatewayChaosTemplate: cleanText(
      readFlag(argv, 'gateway-chaos-template') ||
        'core/local/artifacts/gateway_runtime_chaos_gate_{profile}_current.json',
      400,
    ),
    boundednessBudgetPolicyPath: cleanText(
      readFlag(argv, 'boundedness-budget-policy') ||
        'tests/tooling/config/runtime_boundedness_budgets.json',
      400,
    ),
    boundednessRegressionTolerancePct: Number(
      readFlag(argv, 'boundedness-regression-tolerance-pct') || '0',
    ),
    soakProjectionOutPath: cleanText(
      readFlag(argv, 'soak-projection-out') ||
        'core/local/artifacts/runtime_boundedness_soak_projection_current.json',
      400,
    ),
    soakBootstrapWindowHours: Number(
      readFlag(argv, 'soak-bootstrap-window-hours') || '24',
    ),
    soakTargetWindowHours: Number(
      readFlag(argv, 'soak-target-window-hours') || '72',
    ),
  };
}

function readJson(filePath: string): any {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function readJsonBestEffort(filePath: string): { ok: boolean; payload: any } {
  try {
    return {
      ok: true,
      payload: readJson(filePath),
    };
  } catch (error) {
    return {
      ok: false,
      payload: {
        parse_error: cleanText((error as Error)?.message || 'artifact_parse_error', 220),
      },
    };
  }
}

function parseLastJsonLine(raw: string): any {
  const lines = String(raw || '')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  for (let idx = lines.length - 1; idx >= 0; idx -= 1) {
    try {
      return JSON.parse(lines[idx]);
    } catch {
      // continue
    }
  }
  return null;
}

function shouldRefreshArtifact(
  artifactPath: string,
  revision: string,
): { refresh: boolean; reason: string } {
  if (!fs.existsSync(artifactPath)) {
    return {
      refresh: true,
      reason: 'artifact_missing',
    };
  }
  const parsed = readJsonBestEffort(artifactPath);
  if (!parsed.ok) {
    return {
      refresh: true,
      reason: 'artifact_parse_error',
    };
  }
  const artifactRevision = cleanText(parsed.payload?.revision || '', 80);
  if (!artifactRevision || artifactRevision !== revision) {
    return {
      refresh: true,
      reason: 'revision_mismatch',
    };
  }
  return {
    refresh: false,
    reason: 'artifact_fresh',
  };
}

function runSupportScript(
  root: string,
  scriptPath: string,
  args: string[],
): { status: number; output: any; detail: string } {
  const entrypoint = path.resolve(root, 'client/runtime/lib/ts_entrypoint.ts');
  const script = path.resolve(root, scriptPath);
  try {
    const stdout = execFileSync('node', [entrypoint, script, ...args], {
      cwd: root,
      encoding: 'utf8',
      maxBuffer: 64 * 1024 * 1024,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const output = parseLastJsonLine(String(stdout || ''));
    return {
      status: 0,
      output,
      detail: cleanText(output?.error || 'ok', 320),
    };
  } catch (error) {
    const err = error as {
      status?: number;
      stdout?: string | Buffer;
      stderr?: string | Buffer;
      message?: string;
    };
    const stdout = String(err.stdout || '');
    const stderr = String(err.stderr || '');
    const output = parseLastJsonLine(stdout);
    const status = Number.isFinite(err.status) ? Number(err.status) : 1;
    const detail = cleanText(
      output?.error || err.message || stderr.slice(0, 280) || `status=${status}`,
      320,
    );
    return {
      status,
      output,
      detail,
    };
  }
}

function resolveProfileTemplate(template: string, profile: string): string {
  return cleanText(template.replaceAll('{profile}', profile), 400);
}

function boundednessMetric(report: any, key: string): number {
  const value = Number(report?.summary?.[key]);
  return Number.isFinite(value) ? value : Number.NaN;
}

function toleranceMultiplierFromPct(raw: number): number {
  const pct = Number.isFinite(raw) ? Math.max(0, Number(raw)) : 0;
  return 1 + pct / 100;
}

function isCanonicalToken(raw: string, maxLen = 120): boolean {
  const token = cleanText(String(raw || ''), maxLen);
  return /^[a-z0-9][a-z0-9._:-]*$/i.test(token);
}

function isNonNegativeFiniteNumber(raw: any): boolean {
  const value = Number(raw);
  return Number.isFinite(value) && value >= 0;
}

function isNonNegativeIntegerNumber(raw: any): boolean {
  const value = Number(raw);
  return Number.isInteger(value) && value >= 0;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const revision = currentRevision(root);
  const args = parseArgs(argv);
  const requiredProfiles = ['rich', 'pure', 'tiny-max'];
  const requiredRowMetrics = [
    'peak_rss_mb',
    'storage_usage_mb',
    'queue_depth_max',
    'queue_depth_p95',
    'adapter_restart_count',
    'stale_surface_incidents',
    'conduit_recovery_ms',
  ];
  const regressionDiffMetrics = [
    'max_rss_mb',
    'queue_depth_max',
    'queue_depth_p95',
    'stale_surface_count',
    'adapter_restart_count_max',
    'recovery_time_ms_max',
  ];
  const declaredBudgetMetrics = [...new Set([...regressionDiffMetrics, 'stale_surface_count'])];

  const failures: Array<{ id: string; detail: string }> = [];
  const warnings: Array<{ id: string; detail: string }> = [];
  const boundednessRegressionDiff: Array<{
    profile: string;
    metric: string;
    current: number;
    baseline: number;
    delta: number;
    regression_pct: number | string;
    max_allowed: number;
    tolerance_pct: number;
    ok: boolean;
  }> = [];
  const boundednessBudgetEvaluations: Array<{
    profile: string;
    metric: string;
    current: number;
    budget: number;
    headroom: number;
    ok: boolean;
  }> = [];
  const gatewayQuarantineRecoveryEvidence: Array<{
    profile: string;
    source_artifact: string;
    fail_closed_cases: number;
    transition_cases: number;
    quarantine_events: number;
    recovery_events: number;
    chaos_fail_closed_ratio: number;
    chaos_transition_ratio: number;
    ok: boolean;
  }> = [];
  const supportRuns: Array<{
    id: string;
    script: string;
    refreshed: boolean;
    reason: string;
    status: number;
    detail: string;
  }> = [];

  const dependencyRefreshTargets = [
    {
      id: 'runtime_boundedness_inspect_bundle',
      path: args.boundednessProfilesPath,
      script: 'tests/tooling/scripts/ci/runtime_boundedness_inspect_bundle.ts',
    },
    {
      id: 'queue_backpressure_policy_gate',
      path: args.queueBackpressureGatePath,
      script: 'tests/tooling/scripts/ci/queue_backpressure_policy_gate.ts',
    },
    {
      id: 'dashboard_surface_authority_guard',
      path: args.dashboardSurfaceGuardPath,
      script: 'tests/tooling/scripts/ci/dashboard_surface_authority_guard.ts',
    },
    {
      id: 'layer2_receipt_replay',
      path: args.layer2ReplayPath,
      script: 'tests/tooling/scripts/ci/layer2_receipt_replay.ts',
    },
  ];

  for (const target of dependencyRefreshTargets) {
    const absolute = path.resolve(root, target.path);
    const refreshPlan = shouldRefreshArtifact(absolute, revision);
    if (!refreshPlan.refresh) {
      supportRuns.push({
        id: target.id,
        script: target.script,
        refreshed: false,
        reason: refreshPlan.reason,
        status: 0,
        detail: 'artifact_fresh',
      });
      continue;
    }
    const refresh = runSupportScript(root, target.script, ['--strict=0']);
    supportRuns.push({
      id: target.id,
      script: target.script,
      refreshed: true,
      reason: refreshPlan.reason,
      status: refresh.status,
      detail: refresh.detail,
    });
    if (!fs.existsSync(absolute)) {
      failures.push({
        id: 'dependency_refresh_missing_artifact',
        detail: `${target.id}:${target.path}`,
      });
    } else if (refresh.status !== 0) {
      warnings.push({
        id: 'dependency_refresh_nonzero_exit',
        detail: `${target.id}:status=${refresh.status}`,
      });
    }
  }
  if (supportRuns.length !== dependencyRefreshTargets.length) {
    failures.push({
      id: 'runtime_boundedness_support_runs_count_contract_v2',
      detail: `support_runs=${supportRuns.length};targets=${dependencyRefreshTargets.length}`,
    });
  }
  const supportRunIds = supportRuns
    .map((row) => cleanText(String(row?.id || ''), 120))
    .filter(Boolean);
  if (new Set(supportRunIds).size !== supportRunIds.length) {
    failures.push({
      id: 'runtime_boundedness_support_runs_unique_ids_contract_v2',
      detail: supportRunIds.join(','),
    });
  }
  const allowedSupportRunReasons = new Set([
    'artifact_missing',
    'artifact_parse_error',
    'revision_mismatch',
    'artifact_fresh',
  ]);
  for (const row of supportRuns) {
    if (!isNonNegativeIntegerNumber(row?.status)) {
      failures.push({
        id: 'runtime_boundedness_support_runs_status_scalar_contract_v2',
        detail: `${cleanText(String(row?.id || ''), 80)}:status=${String(row?.status)}`,
      });
    }
    const reason = cleanText(String(row?.reason || ''), 120);
    if (!allowedSupportRunReasons.has(reason)) {
      failures.push({
        id: 'runtime_boundedness_support_runs_reason_token_contract_v2',
        detail: `${cleanText(String(row?.id || ''), 80)}:${reason || 'missing'}`,
      });
    }
    const detail = cleanText(String(row?.detail || ''), 320);
    if (!detail) {
      failures.push({
        id: 'runtime_boundedness_support_runs_detail_nonempty_contract_v2',
        detail: cleanText(String(row?.id || ''), 80) || 'missing_id',
      });
    }
  }

  let boundednessEvidence: any = null;
  let boundednessProfiles: any = null;
  let queueBackpressureGate: any = null;
  let dashboardSurfaceGuard: any = null;
  let layer2Replay: any = null;
  let multiDaySoak: any = null;
  let boundednessBudgetPolicy: any = null;
  try {
    boundednessEvidence = readJson(path.resolve(root, args.boundednessEvidencePath));
  } catch (error) {
    failures.push({
      id: 'boundedness_evidence_missing',
      detail: cleanText(error instanceof Error ? error.message : String(error), 220),
    });
  }
  try {
    boundednessProfiles = readJson(path.resolve(root, args.boundednessProfilesPath));
  } catch (error) {
    failures.push({
      id: 'boundedness_profiles_missing',
      detail: cleanText(error instanceof Error ? error.message : String(error), 220),
    });
  }
  try {
    queueBackpressureGate = readJson(path.resolve(root, args.queueBackpressureGatePath));
  } catch (error) {
    failures.push({
      id: 'queue_backpressure_gate_missing',
      detail: cleanText(error instanceof Error ? error.message : String(error), 220),
    });
  }
  try {
    dashboardSurfaceGuard = readJson(path.resolve(root, args.dashboardSurfaceGuardPath));
  } catch (error) {
    failures.push({
      id: 'dashboard_surface_guard_missing',
      detail: cleanText(error instanceof Error ? error.message : String(error), 220),
    });
  }
  try {
    layer2Replay = readJson(path.resolve(root, args.layer2ReplayPath));
  } catch (error) {
    failures.push({
      id: 'layer2_replay_missing',
      detail: cleanText(error instanceof Error ? error.message : String(error), 220),
    });
  }
  try {
    multiDaySoak = readJson(path.resolve(root, args.multiDaySoakPath));
  } catch (error) {
    failures.push({
      id: 'multi_day_soak_evidence_missing',
      detail: cleanText(error instanceof Error ? error.message : String(error), 220),
    });
  }
  if (multiDaySoak && multiDaySoak?.ok !== true) {
    failures.push({
      id: 'multi_day_soak_evidence_not_ok',
      detail: args.multiDaySoakPath,
    });
  }
  if (
    multiDaySoak &&
    cleanText(String(multiDaySoak?.type || ''), 120) !== 'runtime_multi_day_soak_evidence'
  ) {
    failures.push({
      id: 'multi_day_soak_evidence_type_contract_v3',
      detail: cleanText(String(multiDaySoak?.type || ''), 120) || 'missing',
    });
  }
  try {
    boundednessBudgetPolicy = readJson(path.resolve(root, args.boundednessBudgetPolicyPath));
  } catch (error) {
    failures.push({
      id: 'boundedness_budget_policy_missing',
      detail: cleanText(error instanceof Error ? error.message : String(error), 220),
    });
  }

  if (queueBackpressureGate && queueBackpressureGate?.ok !== true) {
    failures.push({
      id: 'queue_backpressure_gate_not_ok',
      detail: args.queueBackpressureGatePath,
    });
  }
  const queueExpectationChecks = Array.isArray(queueBackpressureGate?.expectation_checks)
    ? queueBackpressureGate.expectation_checks
    : [];
  if (!Array.isArray(queueBackpressureGate?.expectation_checks)) {
    failures.push({
      id: 'runtime_boundedness_queue_expectation_rows_array_contract_v2',
      detail: 'expectation_checks:not_array',
    });
  }
  const queueExpectationBandPresence = new Map<string, boolean>([
    ['defer', false],
    ['shed', false],
    ['quarantine', false],
  ]);
  const allowedQueueExpectationBands = new Set([
    ...queueExpectationBandPresence.keys(),
    'healthy',
  ]);
  for (const row of queueExpectationChecks) {
    if (!row || typeof row !== 'object' || Array.isArray(row)) {
      failures.push({
        id: 'runtime_boundedness_queue_expectation_row_object_contract_v2',
        detail: 'row:not_object',
      });
      continue;
    }
    const band = cleanText(String(row?.expected_band || ''), 80);
    if (!allowedQueueExpectationBands.has(band)) {
      failures.push({
        id: 'runtime_boundedness_queue_expectation_band_token_contract_v2',
        detail: band || 'missing',
      });
    } else if (queueExpectationBandPresence.has(band)) {
      queueExpectationBandPresence.set(band, true);
    }
    const receiptType = cleanText(String(row?.expected_receipt_type || ''), 160);
    if (!isCanonicalToken(receiptType, 160)) {
      failures.push({
        id: 'runtime_boundedness_queue_expectation_receipt_type_token_contract_v2',
        detail: receiptType || 'missing',
      });
    }
    if (typeof row?.ok !== 'boolean') {
      failures.push({
        id: 'runtime_boundedness_queue_expectation_ok_boolean_contract_v2',
        detail: `ok_type=${typeof row?.ok}`,
      });
    }
  }
  for (const [band, present] of queueExpectationBandPresence.entries()) {
    if (!present) {
      failures.push({
        id: 'runtime_boundedness_queue_expectation_band_presence_contract_v2',
        detail: band,
      });
    }
  }
  for (const band of ['defer', 'shed', 'quarantine']) {
    const hasCoveredRow = queueExpectationChecks.some(
      (row: any) =>
        cleanText(String(row?.expected_band || ''), 80) === band &&
        cleanText(String(row?.expected_receipt_type || ''), 160).length > 0 &&
        row?.ok === true,
    );
    if (!hasCoveredRow) {
      failures.push({
        id: 'queue_backpressure_receipt_coverage_missing',
        detail: band,
      });
    }
  }

  if (dashboardSurfaceGuard && dashboardSurfaceGuard?.ok !== true) {
    failures.push({
      id: 'dashboard_surface_guard_not_ok',
      detail: args.dashboardSurfaceGuardPath,
    });
  }
  const dashboardSummary = dashboardSurfaceGuard?.summary || {};
  const runtimeBlockFreshnessFailures = Number(
    dashboardSummary.runtime_block_freshness_contract_failures,
  );
  if (!Number.isFinite(runtimeBlockFreshnessFailures) || runtimeBlockFreshnessFailures !== 0) {
    failures.push({
      id: 'dashboard_runtime_block_freshness_contract_failed',
      detail: `failures=${Number.isFinite(runtimeBlockFreshnessFailures) ? runtimeBlockFreshnessFailures : 'missing'}`,
    });
  }
  const runtimeSyncFreshnessFailures = Number(
    dashboardSummary.runtime_sync_freshness_contract_failures,
  );
  if (!Number.isFinite(runtimeSyncFreshnessFailures) || runtimeSyncFreshnessFailures !== 0) {
    failures.push({
      id: 'dashboard_runtime_sync_freshness_contract_failed',
      detail: `failures=${Number.isFinite(runtimeSyncFreshnessFailures) ? runtimeSyncFreshnessFailures : 'missing'}`,
    });
  }
  if (dashboardSummary.runtime_sync_freshness_summary_consistent !== true) {
    failures.push({
      id: 'dashboard_runtime_sync_freshness_summary_inconsistent',
      detail: `consistent=${String(dashboardSummary.runtime_sync_freshness_summary_consistent)}`,
    });
  }

  if (layer2Replay && layer2Replay?.ok !== true) {
    failures.push({
      id: 'layer2_replay_not_ok',
      detail: args.layer2ReplayPath,
    });
  }
  if (layer2Replay?.auto_heal?.complete !== true) {
    failures.push({
      id: 'conduit_auto_heal_state_machine_incomplete',
      detail: `complete=${String(layer2Replay?.auto_heal?.complete)}`,
    });
  }
  const replayQueueActions = layer2Replay?.queue_backpressure_actions || {};
  for (const action of ['defer_noncritical', 'shed_noncritical', 'quarantine_new_ingress']) {
    if (Number(replayQueueActions?.[action] || 0) <= 0) {
      failures.push({
        id: 'conduit_auto_heal_replay_missing_queue_action',
        detail: action,
      });
    }
  }

  const evidenceProfiles = Array.isArray(boundednessEvidence?.profiles) ? boundednessEvidence.profiles : [];
  const profileReports = Array.isArray(boundednessProfiles?.profiles) ? boundednessProfiles.profiles : [];
  const soakProfiles = Array.isArray(multiDaySoak?.profiles) ? multiDaySoak.profiles : [];
  const soakProfileIds = soakProfiles
    .map((row: any) => cleanText(String(row?.profile || ''), 40))
    .filter(Boolean);
  if (new Set(soakProfileIds).size !== soakProfileIds.length) {
    failures.push({
      id: 'multi_day_soak_profile_rows_unique_contract_v3',
      detail: soakProfileIds.join(',') || 'missing',
    });
  }
  if (soakProfiles.length !== requiredProfiles.length) {
    failures.push({
      id: 'multi_day_soak_profile_count_contract_v3',
      detail: `profiles=${soakProfiles.length};required=${requiredProfiles.length}`,
    });
  }
  const boundednessBudgets: Record<string, any> = {};
  const declaredProfileBudgets =
    boundednessBudgetPolicy?.profiles && typeof boundednessBudgetPolicy.profiles === 'object'
      ? boundednessBudgetPolicy.profiles
      : {};
  const requirePreviousBaseline = boundednessBudgetPolicy?.require_previous_baseline !== false;

  if (!boundednessBudgetPolicy || typeof boundednessBudgetPolicy !== 'object' || Array.isArray(boundednessBudgetPolicy)) {
    failures.push({
      id: 'runtime_boundedness_budget_policy_object_contract_v3',
      detail: args.boundednessBudgetPolicyPath,
    });
  }
  if (boundednessBudgetPolicy && cleanText(String(boundednessBudgetPolicy?.schema_id || ''), 120) !== 'runtime_boundedness_budgets.v1') {
    failures.push({
      id: 'runtime_boundedness_budget_policy_schema_contract_v3',
      detail: cleanText(String(boundednessBudgetPolicy?.schema_id || ''), 120) || 'missing',
    });
  }
  const declaredBudgetProfileNames = Object.keys(declaredProfileBudgets).sort();
  if (declaredBudgetProfileNames.join(',') !== [...requiredProfiles].sort().join(',')) {
    failures.push({
      id: 'runtime_boundedness_declared_budget_profile_set_contract_v3',
      detail: `declared=${declaredBudgetProfileNames.join(',')};required=${requiredProfiles.join(',')}`,
    });
  }

  for (const profile of requiredProfiles) {
    const evidenceRow = evidenceProfiles.find((row: any) => cleanText(row?.profile || '', 40) === profile);
    if (!evidenceRow) {
      failures.push({ id: 'boundedness_evidence_profile_missing', detail: profile });
      continue;
    }
    if (evidenceRow?.scenario_present !== true || evidenceRow?.scenario_ok !== true) {
      failures.push({
        id: 'boundedness_evidence_profile_not_ok',
        detail: `${profile}:present=${String(evidenceRow?.scenario_present)};ok=${String(evidenceRow?.scenario_ok)}`,
      });
    }

    const profileReport = profileReports.find((row: any) => cleanText(row?.profile || '', 40) === profile);
    if (!profileReport) {
      failures.push({ id: 'boundedness_profile_report_missing', detail: profile });
      continue;
    }
    if (profileReport?.ok !== true) {
      failures.push({ id: 'boundedness_profile_report_not_ok', detail: profile });
    }
    const rows = Array.isArray(profileReport?.rows) ? profileReport.rows : [];
    const metricSet = new Set(
      rows.map((row: any) => cleanText(String(row?.metric || ''), 80)).filter(Boolean),
    );
    for (const metric of requiredRowMetrics) {
      if (!metricSet.has(metric)) {
        failures.push({
          id: 'boundedness_required_metric_missing',
          detail: `${profile}:${metric}`,
        });
      }
    }

    const currentBoundednessReportPath = path.resolve(
      root,
      resolveProfileTemplate(args.boundednessReportTemplate, profile),
    );
    let currentBoundednessReport: any = null;
    try {
      currentBoundednessReport = readJson(currentBoundednessReportPath);
    } catch (error) {
      failures.push({
        id: 'boundedness_report_profile_missing',
        detail: `${profile}:${cleanText(error instanceof Error ? error.message : String(error), 220)}`,
      });
    }
    const requiredBoundednessSummaryKeys = [
      'max_rss_mb',
      'queue_depth_max',
      'queue_depth_p95',
      'adapter_restart_count_max',
      'stale_surface_count',
      'recovery_time_ms_max',
    ];
    if (currentBoundednessReport) {
      boundednessBudgets[profile] = {
        max_rss_mb: boundednessMetric(currentBoundednessReport, 'max_rss_mb'),
        queue_depth_max: boundednessMetric(currentBoundednessReport, 'queue_depth_max'),
        queue_depth_p95: boundednessMetric(currentBoundednessReport, 'queue_depth_p95'),
        adapter_restart_count_max: boundednessMetric(currentBoundednessReport, 'adapter_restart_count_max'),
        stale_surface_count: boundednessMetric(currentBoundednessReport, 'stale_surface_count'),
        recovery_time_ms_max: boundednessMetric(currentBoundednessReport, 'recovery_time_ms_max'),
      };
      for (const summaryKey of requiredBoundednessSummaryKeys) {
        const metricValue = boundednessMetric(currentBoundednessReport, summaryKey);
        if (!Number.isFinite(metricValue)) {
          failures.push({
            id: 'boundedness_report_summary_metric_missing',
            detail: `${profile}:${summaryKey}`,
          });
        }
      }
      const declaredBudget = declaredProfileBudgets?.[profile];
      if (!declaredBudget || typeof declaredBudget !== 'object' || Array.isArray(declaredBudget)) {
        failures.push({
          id: 'runtime_boundedness_declared_budget_profile_missing_v3',
          detail: profile,
        });
      } else {
        for (const summaryKey of declaredBudgetMetrics) {
          const budgetValue = Number(declaredBudget?.[summaryKey]);
          if (!isNonNegativeFiniteNumber(budgetValue)) {
            failures.push({
              id: 'runtime_boundedness_declared_budget_metric_contract_v3',
              detail: `${profile}:${summaryKey}:${String(declaredBudget?.[summaryKey])}`,
            });
            continue;
          }
          const currentValue = boundednessMetric(currentBoundednessReport, summaryKey);
          if (!Number.isFinite(currentValue)) continue;
          const headroom = Number((budgetValue - currentValue).toFixed(6));
          const ok = currentValue <= budgetValue;
          boundednessBudgetEvaluations.push({
            profile,
            metric: summaryKey,
            current: currentValue,
            budget: budgetValue,
            headroom,
            ok,
          });
          if (!ok) {
            failures.push({
              id: 'boundedness_budget_exceeded',
              detail: `${profile}:${summaryKey}:current=${currentValue};budget=${budgetValue}`,
            });
          }
        }
        if (
          isNonNegativeFiniteNumber(declaredBudget?.queue_depth_p95) &&
          isNonNegativeFiniteNumber(declaredBudget?.queue_depth_max) &&
          Number(declaredBudget.queue_depth_p95) > Number(declaredBudget.queue_depth_max)
        ) {
          failures.push({
            id: 'runtime_boundedness_declared_budget_queue_p95_leq_max_contract_v3',
            detail: `${profile}:p95=${Number(declaredBudget.queue_depth_p95)};max=${Number(declaredBudget.queue_depth_max)}`,
          });
        }
        if (
          Number.isFinite(Number(declaredBudget?.stale_surface_count)) &&
          !Number.isInteger(Number(declaredBudget.stale_surface_count))
        ) {
          failures.push({
            id: 'runtime_boundedness_declared_budget_stale_surface_integer_contract_v3',
            detail: `${profile}:stale_surface_count=${String(declaredBudget.stale_surface_count)}`,
          });
        }
      }
    }

    const baselineBoundednessReportPath = path.resolve(
      root,
      resolveProfileTemplate(args.boundednessBaselineTemplate, profile),
    );
    let baselineBoundednessReport: any = null;
    if (fs.existsSync(baselineBoundednessReportPath)) {
      try {
        baselineBoundednessReport = readJson(baselineBoundednessReportPath);
      } catch (error) {
        failures.push({
          id: 'boundedness_baseline_report_read_failed',
          detail: `${profile}:${cleanText(error instanceof Error ? error.message : String(error), 220)}`,
        });
      }
    } else {
      const missingBaseline = {
        id: 'boundedness_baseline_report_missing',
        detail: `${profile}:${path.relative(root, baselineBoundednessReportPath)}`,
      };
      if (requirePreviousBaseline) {
        failures.push(missingBaseline);
      } else {
        warnings.push(missingBaseline);
      }
    }
    if (currentBoundednessReport && baselineBoundednessReport) {
      const tolerancePct = Number.isFinite(args.boundednessRegressionTolerancePct)
        ? Math.max(0, args.boundednessRegressionTolerancePct)
        : 0;
      const toleranceMultiplier = toleranceMultiplierFromPct(tolerancePct);
      for (const summaryKey of regressionDiffMetrics) {
        const currentValue = boundednessMetric(currentBoundednessReport, summaryKey);
        const baselineValue = boundednessMetric(baselineBoundednessReport, summaryKey);
        if (!Number.isFinite(currentValue) || !Number.isFinite(baselineValue)) continue;
        const maxAllowed = baselineValue * toleranceMultiplier;
        const delta = currentValue - baselineValue;
        const regressionPct =
          baselineValue > 0
            ? Number(((delta / baselineValue) * 100).toFixed(6))
            : currentValue > baselineValue
              ? 'infinite'
              : 0;
        const ok = currentValue <= maxAllowed;
        boundednessRegressionDiff.push({
          profile,
          metric: summaryKey,
          current: currentValue,
          baseline: baselineValue,
          delta: Number(delta.toFixed(6)),
          regression_pct: regressionPct,
          max_allowed: Number(maxAllowed.toFixed(6)),
          tolerance_pct: tolerancePct,
          ok,
        });
        if (!ok) {
          failures.push({
            id: 'boundedness_regression_detected',
            detail: `${profile}:${summaryKey}:current=${currentValue};baseline=${baselineValue};max_allowed=${maxAllowed}`,
          });
        }
      }
    }

    const harnessPath = path.resolve(
      root,
      args.harnessRootPath,
      `runtime_proof_harness_${profile}_current.json`,
    );
    let harness: any = null;
    try {
      harness = readJson(harnessPath);
    } catch (error) {
      failures.push({
        id: 'conduit_auto_heal_harness_profile_missing',
        detail: `${profile}:${cleanText(error instanceof Error ? error.message : String(error), 220)}`,
      });
    }
    const deterministicChecksum = cleanText(String(harness?.deterministic_checksum || ''), 120);
    if (!/^[a-f0-9]{64}$/i.test(deterministicChecksum)) {
      failures.push({
        id: 'conduit_auto_heal_harness_checksum_invalid',
        detail: `${profile}:${deterministicChecksum || 'missing'}`,
      });
    }
    const harnessScenarios = Array.isArray(harness?.scenarios) ? harness.scenarios : [];
    const conduitRecoveryScenario = harnessScenarios.find(
      (row: any) => cleanText(String(row?.id || ''), 80) === 'conduit_failure_recovery',
    );
    if (!conduitRecoveryScenario) {
      failures.push({
        id: 'conduit_auto_heal_harness_scenario_missing',
        detail: `${profile}:conduit_failure_recovery`,
      });
    } else {
    if (conduitRecoveryScenario?.ok !== true) {
        failures.push({
          id: 'conduit_auto_heal_harness_scenario_not_ok',
          detail: profile,
        });
      }
      if (Number(conduitRecoveryScenario?.metrics?.conduit_recovery_ms || 0) <= 0) {
        failures.push({
          id: 'conduit_auto_heal_harness_scenario_invalid_metric',
          detail: `${profile}:conduit_recovery_ms<=0`,
        });
      }
    }

    const soakProfile = soakProfiles.find((row: any) => cleanText(row?.profile || '', 40) === profile);
    if (!soakProfile) {
      failures.push({
        id: 'multi_day_soak_profile_missing',
        detail: profile,
      });
    } else {
      const soakSamples = Number(soakProfile?.soak_source_sample_points || 0);
      const empiricalSamples = Number(soakProfile?.empirical_sample_points || 0);
      const sourceArtifact = cleanText(String(soakProfile?.source_artifact || ''), 400);
      if (soakSamples <= 0 && empiricalSamples <= 0) {
        failures.push({
          id: 'multi_day_soak_profile_sample_points_missing',
          detail: profile,
        });
      }
      if (!sourceArtifact || !/^[a-z0-9][a-z0-9._:/-]*$/i.test(sourceArtifact)) {
        failures.push({
          id: 'multi_day_soak_source_artifact_contract_v3',
          detail: `${profile}:${sourceArtifact || 'missing'}`,
        });
      }
    }

    const gatewayChaosPath = resolveProfileTemplate(args.gatewayChaosTemplate, profile);
    let gatewayChaos: any = null;
    try {
      gatewayChaos = readJson(path.resolve(root, gatewayChaosPath));
    } catch (error) {
      failures.push({
        id: 'gateway_quarantine_recovery_artifact_missing',
        detail: `${profile}:${cleanText(error instanceof Error ? error.message : String(error), 220)}`,
      });
    }
    if (gatewayChaos) {
      const chaosResults = Array.isArray(gatewayChaos?.chaos_results)
        ? gatewayChaos.chaos_results
        : [];
      const transitionResults = Array.isArray(gatewayChaos?.chaos_transition_results)
        ? gatewayChaos.chaos_transition_results
        : [];
      const quarantineRows = transitionResults.filter(
        (row: any) =>
          cleanText(String(row?.scenario || ''), 80) === 'repeated_flapping' &&
          row?.transition_ok === true &&
          cleanText(String(row?.runtime_circuit_state || ''), 40) === 'open' &&
          row?.runtime_quarantine_active === true,
      );
      const recoveryRows = transitionResults.filter((row: any) => row?.transition_ok === true);
      const failClosedRows = chaosResults.filter((row: any) => row?.ok === true);
      const chaosFailClosedRatio = Number(gatewayChaos?.summary?.chaos_fail_closed_ratio || 0);
      const chaosTransitionRatio = Number(gatewayChaos?.summary?.chaos_transition_ratio || 0);
      const gatewayOk =
        gatewayChaos?.ok === true &&
        chaosFailClosedRatio >= 1 &&
        chaosTransitionRatio >= 1 &&
        quarantineRows.length > 0 &&
        recoveryRows.length > 0;
      gatewayQuarantineRecoveryEvidence.push({
        profile,
        source_artifact: gatewayChaosPath,
        fail_closed_cases: failClosedRows.length,
        transition_cases: transitionResults.length,
        quarantine_events: quarantineRows.length,
        recovery_events: recoveryRows.length,
        chaos_fail_closed_ratio: chaosFailClosedRatio,
        chaos_transition_ratio: chaosTransitionRatio,
        ok: gatewayOk,
      });
      if (!gatewayOk) {
        failures.push({
          id: 'gateway_quarantine_recovery_evidence_not_ok',
          detail: `${profile}:fail_closed_ratio=${chaosFailClosedRatio};transition_ratio=${chaosTransitionRatio};quarantine=${quarantineRows.length};recovery=${recoveryRows.length}`,
        });
      }
    }
  }
  if (Object.keys(boundednessBudgets).length !== requiredProfiles.length) {
    failures.push({
      id: 'runtime_boundedness_budget_profiles_count_contract_v2',
      detail: `budgets=${Object.keys(boundednessBudgets).length};required=${requiredProfiles.length}`,
    });
  }
  for (const profile of requiredProfiles) {
    const budget = boundednessBudgets[profile];
    if (!budget || typeof budget !== 'object') continue;
    const metricPairs = [
      ['max_rss_mb', budget.max_rss_mb],
      ['queue_depth_max', budget.queue_depth_max],
      ['queue_depth_p95', budget.queue_depth_p95],
      ['adapter_restart_count_max', budget.adapter_restart_count_max],
      ['stale_surface_count', budget.stale_surface_count],
      ['recovery_time_ms_max', budget.recovery_time_ms_max],
    ];
    for (const [metric, value] of metricPairs) {
      if (!isNonNegativeFiniteNumber(value)) {
        failures.push({
          id: 'runtime_boundedness_budget_metric_non_negative_contract_v2',
          detail: `${profile}:${metric}:${String(value)}`,
        });
      }
    }
    if (
      isNonNegativeFiniteNumber(budget.queue_depth_p95) &&
      isNonNegativeFiniteNumber(budget.queue_depth_max) &&
      Number(budget.queue_depth_p95) > Number(budget.queue_depth_max)
    ) {
      failures.push({
        id: 'runtime_boundedness_budget_queue_p95_leq_max_contract_v2',
        detail: `${profile}:p95=${Number(budget.queue_depth_p95)};max=${Number(budget.queue_depth_max)}`,
      });
    }
    if (
      Number.isFinite(Number(budget.stale_surface_count)) &&
      !Number.isInteger(Number(budget.stale_surface_count))
    ) {
      failures.push({
        id: 'runtime_boundedness_budget_stale_surface_integer_contract_v2',
        detail: `${profile}:stale_surface_count=${String(budget.stale_surface_count)}`,
      });
    }
  }

  const soakBootstrapWindowHours = Math.max(
    1,
    Number.isFinite(args.soakBootstrapWindowHours) ? Number(args.soakBootstrapWindowHours) : 24,
  );
  const soakTargetWindowHours = Math.max(
    soakBootstrapWindowHours,
    Number.isFinite(args.soakTargetWindowHours) ? Number(args.soakTargetWindowHours) : 72,
  );
  const soakProjectionProfiles = requiredProfiles.map((profile) => {
    const soakRow = soakProfiles.find((row: any) => cleanText(row?.profile || '', 40) === profile) || {};
    const observedSamplePoints = Number(soakRow?.soak_source_sample_points || 0);
    const empiricalSamplePoints = Number(soakRow?.empirical_sample_points || 0);
    const projected72hSamplePoints =
      observedSamplePoints > 0
        ? Number(
            (observedSamplePoints * (soakTargetWindowHours / soakBootstrapWindowHours)).toFixed(3),
          )
        : 0;
    const projected72hReady =
      (soakRow?.soak_source_loaded === true || empiricalSamplePoints > 0) &&
      projected72hSamplePoints > 0;
    return {
      profile,
      soak_source_loaded: soakRow?.soak_source_loaded === true,
      observed_sample_points: observedSamplePoints,
      empirical_sample_points: empiricalSamplePoints,
      bootstrap_window_hours: soakBootstrapWindowHours,
      target_window_hours: soakTargetWindowHours,
      projected_72h_sample_points: projected72hSamplePoints,
      projected_72h_ready: projected72hReady,
      source_artifact: cleanText(String(soakRow?.source_artifact || ''), 220),
    };
  });
  const soakProjection = {
    ok: soakProjectionProfiles.every((row) => row.projected_72h_ready === true),
    type: 'runtime_boundedness_soak_projection',
    generated_at: new Date().toISOString(),
    revision,
    source_artifact: args.multiDaySoakPath,
    bootstrap_window_hours: soakBootstrapWindowHours,
    target_window_hours: soakTargetWindowHours,
    profiles: soakProjectionProfiles,
  };
  const soakProjectionProfilesSeen = soakProjectionProfiles.map((row) =>
    cleanText(String(row?.profile || ''), 40),
  );
  if (new Set(soakProjectionProfilesSeen).size !== soakProjectionProfilesSeen.length) {
    failures.push({
      id: 'runtime_boundedness_soak_projection_profile_rows_unique_contract_v2',
      detail: soakProjectionProfilesSeen.join(','),
    });
  }
  if (soakProjectionProfiles.length !== requiredProfiles.length) {
    failures.push({
      id: 'runtime_boundedness_soak_projection_profile_count_contract_v2',
      detail: `profiles=${soakProjectionProfiles.length};required=${requiredProfiles.length}`,
    });
  }
  for (const row of soakProjectionProfiles) {
    const profile = cleanText(String(row?.profile || ''), 40) || 'unknown';
    const observed = Number(row?.observed_sample_points || 0);
    const empirical = Number(row?.empirical_sample_points || 0);
    const projected = Number(row?.projected_72h_sample_points || 0);
    if (
      !isNonNegativeFiniteNumber(observed) ||
      !isNonNegativeFiniteNumber(empirical) ||
      !isNonNegativeFiniteNumber(projected)
    ) {
      failures.push({
        id: 'runtime_boundedness_soak_projection_scalar_non_negative_contract_v2',
        detail: `${profile}:observed=${String(observed)};empirical=${String(empirical)};projected=${String(projected)}`,
      });
    }
    if (row?.soak_source_loaded === true && !cleanText(String(row?.source_artifact || ''), 220)) {
      failures.push({
        id: 'runtime_boundedness_soak_projection_source_artifact_when_loaded_contract_v2',
        detail: profile,
      });
    }
    if (
      observed > 0 &&
      projected < observed &&
      soakTargetWindowHours >= soakBootstrapWindowHours
    ) {
      failures.push({
        id: 'runtime_boundedness_soak_projection_projection_monotonic_contract_v2',
        detail: `${profile}:observed=${observed};projected=${projected}`,
      });
    }
  }
  const soakProjectionPath = path.resolve(root, args.soakProjectionOutPath);
  fs.mkdirSync(path.dirname(soakProjectionPath), { recursive: true });
  fs.writeFileSync(soakProjectionPath, `${JSON.stringify(soakProjection, null, 2)}\n`, 'utf8');
  if (soakProjection.ok !== true) {
    failures.push({
      id: 'boundedness_soak_projection_not_ok',
      detail: `projection_path=${args.soakProjectionOutPath}`,
    });
  }
  if (gatewayQuarantineRecoveryEvidence.length !== requiredProfiles.length) {
    failures.push({
      id: 'gateway_quarantine_recovery_profile_count_contract_v3',
      detail: `profiles=${gatewayQuarantineRecoveryEvidence.length};required=${requiredProfiles.length}`,
    });
  }

  const report = {
    ok: failures.length === 0,
    type: 'runtime_boundedness_release_gate',
    generated_at: new Date().toISOString(),
    revision,
    inputs: {
      boundedness_evidence_path: args.boundednessEvidencePath,
      boundedness_profiles_path: args.boundednessProfilesPath,
      queue_backpressure_gate_path: args.queueBackpressureGatePath,
      dashboard_surface_guard_path: args.dashboardSurfaceGuardPath,
      layer2_replay_path: args.layer2ReplayPath,
      multi_day_soak_path: args.multiDaySoakPath,
      soak_projection_path: args.soakProjectionOutPath,
      harness_root_path: args.harnessRootPath,
      boundedness_report_template: args.boundednessReportTemplate,
      boundedness_baseline_template: args.boundednessBaselineTemplate,
      boundedness_budget_policy_path: args.boundednessBudgetPolicyPath,
      gateway_chaos_template: args.gatewayChaosTemplate,
      boundedness_regression_tolerance_pct: args.boundednessRegressionTolerancePct,
      soak_bootstrap_window_hours: soakBootstrapWindowHours,
      soak_target_window_hours: soakTargetWindowHours,
    },
    summary: {
      required_profiles: requiredProfiles,
      required_metric_rows: requiredRowMetrics,
      regression_diff_metrics: regressionDiffMetrics,
      queue_backpressure_receipt_coverage_profiles: ['defer', 'shed', 'quarantine'],
      boundedness_regression_failure_count: failures.filter(
        (row) => row.id === 'boundedness_regression_detected',
      ).length,
      boundedness_regression_diff_count: boundednessRegressionDiff.length,
      boundedness_budget_failure_count: failures.filter(
        (row) => row.id === 'boundedness_budget_exceeded',
      ).length,
      boundedness_budget_evaluation_count: boundednessBudgetEvaluations.length,
      boundedness_budget_profile_count: Object.keys(boundednessBudgets).length,
      declared_boundedness_budget_profile_count: declaredBudgetProfileNames.length,
      require_previous_baseline: requirePreviousBaseline,
      soak_projection_profile_count: soakProjectionProfiles.length,
      soak_projection_pass: soakProjection.ok === true,
      multi_day_soak_profile_count: soakProfiles.length,
      gateway_quarantine_recovery_profile_count: gatewayQuarantineRecoveryEvidence.length,
      gateway_quarantine_recovery_pass: gatewayQuarantineRecoveryEvidence.every((row) => row.ok),
      support_runs_total: supportRuns.length,
      support_runs_refreshed: supportRuns.filter((row) => row.refreshed).length,
      failed_count: failures.length,
      warning_count: warnings.length,
      pass: failures.length === 0,
    },
    boundedness_budgets: {
      regression_tolerance_pct: args.boundednessRegressionTolerancePct,
      profiles: boundednessBudgets,
    },
    declared_boundedness_budgets: {
      schema_id: cleanText(String(boundednessBudgetPolicy?.schema_id || ''), 120),
      require_previous_baseline: requirePreviousBaseline,
      profiles: declaredProfileBudgets,
    },
    boundedness_regression_diff: boundednessRegressionDiff,
    boundedness_budget_evaluations: boundednessBudgetEvaluations,
    gateway_quarantine_recovery_evidence: gatewayQuarantineRecoveryEvidence,
    soak_projection: soakProjection,
    support_runs: supportRuns,
    artifact_paths: [args.soakProjectionOutPath],
    failures,
    warnings,
  };

  return emitStructuredResult(report, {
    outPath: args.outPath,
    strict: args.strict,
    ok: report.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
