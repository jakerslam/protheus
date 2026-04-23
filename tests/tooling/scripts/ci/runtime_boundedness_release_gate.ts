#!/usr/bin/env tsx

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

function resolveProfileTemplate(template: string, profile: string): string {
  return cleanText(template.replaceAll('{profile}', profile), 400);
}

function boundednessMetric(report: any, key: string): number {
  const value = Number(report?.summary?.[key]);
  return Number.isFinite(value) ? value : Number.NaN;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const requiredProfiles = ['rich', 'pure', 'tiny-max'];
  const requiredRowMetrics = [
    'peak_rss_mb',
    'storage_usage_mb',
    'queue_depth_max',
    'queue_depth_p95',
    'stale_surface_incidents',
    'conduit_recovery_ms',
  ];

  const failures: Array<{ id: string; detail: string }> = [];
  const warnings: Array<{ id: string; detail: string }> = [];
  let boundednessEvidence: any = null;
  let boundednessProfiles: any = null;
  let queueBackpressureGate: any = null;
  let dashboardSurfaceGuard: any = null;
  let layer2Replay: any = null;
  let multiDaySoak: any = null;
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

  if (queueBackpressureGate && queueBackpressureGate?.ok !== true) {
    failures.push({
      id: 'queue_backpressure_gate_not_ok',
      detail: args.queueBackpressureGatePath,
    });
  }
  const queueExpectationChecks = Array.isArray(queueBackpressureGate?.expectation_checks)
    ? queueBackpressureGate.expectation_checks
    : [];
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
  if (Number(dashboardSummary.runtime_block_freshness_contract_failures || 1) !== 0) {
    failures.push({
      id: 'dashboard_runtime_block_freshness_contract_failed',
      detail: `failures=${Number(dashboardSummary.runtime_block_freshness_contract_failures || 0)}`,
    });
  }
  if (Number(dashboardSummary.runtime_sync_freshness_contract_failures || 1) !== 0) {
    failures.push({
      id: 'dashboard_runtime_sync_freshness_contract_failed',
      detail: `failures=${Number(dashboardSummary.runtime_sync_freshness_contract_failures || 0)}`,
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
  const boundednessBudgets: Record<string, any> = {};

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
      'stale_surface_count',
      'recovery_time_ms_max',
    ];
    if (currentBoundednessReport) {
      boundednessBudgets[profile] = {
        max_rss_mb: boundednessMetric(currentBoundednessReport, 'max_rss_mb'),
        queue_depth_max: boundednessMetric(currentBoundednessReport, 'queue_depth_max'),
        queue_depth_p95: boundednessMetric(currentBoundednessReport, 'queue_depth_p95'),
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
      warnings.push({
        id: 'boundedness_baseline_report_missing',
        detail: `${profile}:${path.relative(root, baselineBoundednessReportPath)}`,
      });
    }
    if (currentBoundednessReport && baselineBoundednessReport) {
      const toleranceMultiplier = 1 + Math.max(0, Number.isFinite(args.boundednessRegressionTolerancePct) ? args.boundednessRegressionTolerancePct : 0);
      for (const summaryKey of requiredBoundednessSummaryKeys) {
        const currentValue = boundednessMetric(currentBoundednessReport, summaryKey);
        const baselineValue = boundednessMetric(baselineBoundednessReport, summaryKey);
        if (!Number.isFinite(currentValue) || !Number.isFinite(baselineValue)) continue;
        const maxAllowed = baselineValue * toleranceMultiplier;
        if (currentValue > maxAllowed) {
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
      if (soakSamples <= 0 && empiricalSamples <= 0) {
        failures.push({
          id: 'multi_day_soak_profile_sample_points_missing',
          detail: profile,
        });
      }
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
    revision: currentRevision(root),
    source_artifact: args.multiDaySoakPath,
    bootstrap_window_hours: soakBootstrapWindowHours,
    target_window_hours: soakTargetWindowHours,
    profiles: soakProjectionProfiles,
  };
  const soakProjectionPath = path.resolve(root, args.soakProjectionOutPath);
  fs.mkdirSync(path.dirname(soakProjectionPath), { recursive: true });
  fs.writeFileSync(soakProjectionPath, `${JSON.stringify(soakProjection, null, 2)}\n`, 'utf8');
  if (soakProjection.ok !== true) {
    failures.push({
      id: 'boundedness_soak_projection_not_ok',
      detail: `projection_path=${args.soakProjectionOutPath}`,
    });
  }

  const report = {
    ok: failures.length === 0,
    type: 'runtime_boundedness_release_gate',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
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
      boundedness_regression_tolerance_pct: args.boundednessRegressionTolerancePct,
      soak_bootstrap_window_hours: soakBootstrapWindowHours,
      soak_target_window_hours: soakTargetWindowHours,
    },
    summary: {
      required_profiles: requiredProfiles,
      required_metric_rows: requiredRowMetrics,
      queue_backpressure_receipt_coverage_profiles: ['defer', 'shed', 'quarantine'],
      boundedness_regression_failure_count: failures.filter(
        (row) => row.id === 'boundedness_regression_detected',
      ).length,
      boundedness_budget_profile_count: Object.keys(boundednessBudgets).length,
      soak_projection_profile_count: soakProjectionProfiles.length,
      soak_projection_pass: soakProjection.ok === true,
      failed_count: failures.length,
      warning_count: warnings.length,
      pass: failures.length === 0,
    },
    boundedness_budgets: {
      regression_tolerance_pct: args.boundednessRegressionTolerancePct,
      profiles: boundednessBudgets,
    },
    soak_projection: soakProjection,
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
