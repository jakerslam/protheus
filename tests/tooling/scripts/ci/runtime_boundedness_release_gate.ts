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
    harnessRootPath: cleanText(
      readFlag(argv, 'harness-root') || 'core/local/artifacts',
      400,
    ),
  };
}

function readJson(filePath: string): any {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
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
  let boundednessEvidence: any = null;
  let boundednessProfiles: any = null;
  let queueBackpressureGate: any = null;
  let dashboardSurfaceGuard: any = null;
  let layer2Replay: any = null;
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
      harness_root_path: args.harnessRootPath,
    },
    summary: {
      required_profiles: requiredProfiles,
      required_metric_rows: requiredRowMetrics,
      queue_backpressure_receipt_coverage_profiles: ['defer', 'shed', 'quarantine'],
      failed_count: failures.length,
      pass: failures.length === 0,
    },
    failures,
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
