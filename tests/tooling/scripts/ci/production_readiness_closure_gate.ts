#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { executeGate } from '../../lib/runner.ts';

type Args = {
  strict: boolean;
  out: string;
  runSmoke: boolean;
  stage: 'prebundle' | 'final';
  supportBundlePath: string;
  scorecardPath: string;
  topologyPath: string;
  stateCompatPath: string;
  rcRehearsalPath: string;
  clientBoundaryPath: string;
};

type Check = {
  id: string;
  ok: boolean;
  detail: string;
};

type Policy = {
  required_files?: string[];
  required_package_scripts?: string[];
  required_ci_invocations?: string[];
  required_verify_invocations?: string[];
  required_verify_profile_gate_ids?: Record<string, string[]>;
  required_readme_markers?: string[];
  smoke_scripts?: string[];
  numeric_thresholds?: {
    ipc_success_rate_min?: number;
    receipt_completeness_rate_min?: number;
    supported_command_latency_ms_max?: number;
    recovery_rto_minutes_max?: number;
    recovery_rpo_hours_max?: number;
  };
  release_candidate_rehearsal?: {
    required_step_gate_ids?: string[];
  };
  standing_regression_guards?: {
    client_authority_gate_id?: string;
  };
};

const ROOT = process.cwd();
const POLICY_PATH = path.join(ROOT, 'client/runtime/config/production_readiness_closure_policy.json');
const VERIFY_PROFILES_PATH = path.join(ROOT, 'tests/tooling/config/verify_profiles.json');
const GATE_REGISTRY_PATH = 'tests/tooling/config/tooling_gate_registry.json';
const TOPOLOGY_ARTIFACT_PATH = path.join(ROOT, 'core/local/artifacts/production_topology_diagnostic_current.json');
const STATE_COMPAT_ARTIFACT_PATH = path.join(ROOT, 'core/local/artifacts/stateful_upgrade_rollback_gate_current.json');
const SUPPORT_BUNDLE_ARTIFACT_PATH = path.join(ROOT, 'core/local/artifacts/support_bundle_latest.json');
const RELEASE_SCORECARD_PATH = path.join(
  ROOT,
  'client/runtime/local/state/release/scorecard/release_scorecard.json',
);
const RELEASE_RC_REHEARSAL_PATH = path.join(
  ROOT,
  'core/local/artifacts/release_candidate_dress_rehearsal_current.json',
);
const CLIENT_BOUNDARY_ARTIFACT_PATH = path.join(
  ROOT,
  'core/local/artifacts/client_layer_boundary_audit_current.json',
);

function parseBool(raw: string | undefined, fallback = false): boolean {
  const value = String(raw || '').trim().toLowerCase();
  if (!value) return fallback;
  return value === '1' || value === 'true' || value === 'yes' || value === 'on';
}

function parseArgs(argv: string[]): Args {
  const args: Args = {
    strict: false,
    out: 'core/local/artifacts/production_readiness_closure_gate_current.json',
    runSmoke: true,
    stage: 'final',
    supportBundlePath: SUPPORT_BUNDLE_ARTIFACT_PATH,
    scorecardPath: RELEASE_SCORECARD_PATH,
    topologyPath: TOPOLOGY_ARTIFACT_PATH,
    stateCompatPath: STATE_COMPAT_ARTIFACT_PATH,
    rcRehearsalPath: RELEASE_RC_REHEARSAL_PATH,
    clientBoundaryPath: CLIENT_BOUNDARY_ARTIFACT_PATH,
  };
  for (const token of argv) {
    if (token === '--strict') args.strict = true;
    else if (token.startsWith('--strict=')) args.strict = parseBool(token.slice('--strict='.length), false);
    else if (token.startsWith('--out=')) args.out = token.slice('--out='.length);
    else if (token.startsWith('--run-smoke=')) args.runSmoke = parseBool(token.slice('--run-smoke='.length), true);
    else if (token.startsWith('--stage=')) {
      args.stage = token.slice('--stage='.length) === 'prebundle' ? 'prebundle' : 'final';
    }
    else if (token.startsWith('--support-bundle=')) {
      args.supportBundlePath = path.resolve(ROOT, token.slice('--support-bundle='.length));
    }
    else if (token.startsWith('--scorecard=')) {
      args.scorecardPath = path.resolve(ROOT, token.slice('--scorecard='.length));
    }
    else if (token.startsWith('--topology=')) {
      args.topologyPath = path.resolve(ROOT, token.slice('--topology='.length));
    }
    else if (token.startsWith('--state-compat=')) {
      args.stateCompatPath = path.resolve(ROOT, token.slice('--state-compat='.length));
    }
    else if (token.startsWith('--rc-rehearsal=')) {
      args.rcRehearsalPath = path.resolve(ROOT, token.slice('--rc-rehearsal='.length));
    }
    else if (token.startsWith('--client-boundary=')) {
      args.clientBoundaryPath = path.resolve(ROOT, token.slice('--client-boundary='.length));
    }
  }
  return args;
}

function readJson<T>(filePath: string, fallback: T): T {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8')) as T;
  } catch {
    return fallback;
  }
}

function safeNumber(value: unknown, fallback: number): number {
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric : fallback;
}

function metricText(value: unknown): string {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return 'missing';
  return Number.isInteger(numeric) ? String(numeric) : numeric.toFixed(4);
}

function checkRequiredFiles(files: string[]): Check[] {
  return files.map((relPath) => {
    const ok = fs.existsSync(path.resolve(ROOT, relPath));
    return {
      id: `required_file:${relPath}`,
      ok,
      detail: ok ? 'present' : 'missing',
    };
  });
}

function checkPackageScripts(requiredScripts: string[]): Check[] {
  const pkg = readJson<{ scripts?: Record<string, string> }>(path.join(ROOT, 'package.json'), {});
  const scripts = pkg.scripts || {};
  return requiredScripts.map((scriptName) => {
    const ok = typeof scripts[scriptName] === 'string' && scripts[scriptName].trim().length > 0;
    return {
      id: `package_script:${scriptName}`,
      ok,
      detail: ok ? 'registered' : 'missing',
    };
  });
}

function checkTextMarkers(filePath: string, markers: string[], prefix: string): Check[] {
  let source = '';
  try {
    source = fs.readFileSync(filePath, 'utf8');
  } catch {
    return markers.map((marker) => ({
      id: `${prefix}:${marker}`,
      ok: false,
      detail: `${path.relative(ROOT, filePath)} missing`,
    }));
  }
  return markers.map((marker) => {
    const ok = source.includes(marker);
    return {
      id: `${prefix}:${marker}`,
      ok,
      detail: ok ? 'present' : 'missing',
    };
  });
}

function checkWorkflowMarkers(markers: string[]): Check[] {
  const workflowFiles = [
    path.join(ROOT, '.github/workflows/ci.yml'),
    path.join(ROOT, '.github/workflows/release.yml'),
  ];
  const sources = workflowFiles
    .filter((filePath) => fs.existsSync(filePath))
    .map((filePath) => fs.readFileSync(filePath, 'utf8'));
  return markers.map((marker) => {
    const ok = sources.some((source) => source.includes(marker));
    return {
      id: `ci_invocation:${marker}`,
      ok,
      detail: ok ? 'present' : 'missing_from_ci_or_release_workflow',
    };
  });
}

function checkVerifyProfileGateIds(requiredProfiles: Record<string, string[]>): Check[] {
  const manifest = readJson<{ profiles?: Record<string, { gate_ids?: string[] }> }>(
    VERIFY_PROFILES_PATH,
    {},
  );
  const profiles = manifest.profiles || {};
  const checks: Check[] = [];
  for (const [profileId, requiredGateIds] of Object.entries(requiredProfiles || {})) {
    const gateIds = Array.isArray(profiles[profileId]?.gate_ids) ? profiles[profileId]?.gate_ids || [] : [];
    if (!profiles[profileId]) {
      checks.push({
        id: `verify_profile:${profileId}`,
        ok: false,
        detail: 'missing',
      });
      continue;
    }
    for (const gateId of requiredGateIds) {
      const ok = gateIds.includes(gateId);
      checks.push({
        id: `verify_profile_gate:${profileId}:${gateId}`,
        ok,
        detail: ok ? 'present' : 'missing',
      });
    }
  }
  return checks;
}

function runSmokeScripts(scriptNames: string[]): Check[] {
  return scriptNames.map((scriptName) => {
    try {
      const report = executeGate(scriptName, {
        registryPath: GATE_REGISTRY_PATH,
        strict: true,
      });
      return {
        id: `smoke_script:${scriptName}`,
        ok: report.ok,
        detail: report.ok
          ? `ok:${report.summary.exit_code}`
          : String(report.failures[0]?.detail || 'gate_failed').slice(0, 400),
      };
    } catch (error) {
      return {
        id: `smoke_script:${scriptName}`,
        ok: false,
        detail: `unregistered_or_failed:${String(error)}`.slice(0, 400),
      };
    }
  });
}

function checkReleaseEvidence(policy: Policy, args: Args): Check[] {
  const topology = readJson<any>(args.topologyPath, {});
  const stateCompat = readJson<any>(args.stateCompatPath, {});
  const supportBundle = readJson<any>(args.supportBundlePath, {});
  const scorecard = readJson<any>(args.scorecardPath, {});
  const rcRehearsal = readJson<any>(args.rcRehearsalPath, {});
  const clientBoundary = readJson<any>(args.clientBoundaryPath, {});
  const thresholds = policy.numeric_thresholds || {};
  const finalStage = args.stage === 'final';
  const requiredRcStepIds = Array.isArray(policy.release_candidate_rehearsal?.required_step_gate_ids)
    ? policy.release_candidate_rehearsal?.required_step_gate_ids || []
    : [];
  const clientAuthorityGateId = String(
    policy.standing_regression_guards?.client_authority_gate_id || 'audit:client-layer-boundary',
  );
  const scorecardGates = new Map<string, any>(
    (Array.isArray(scorecard?.gates) ? scorecard.gates : []).map((row: any) => [String(row?.id || ''), row]),
  );
  const liveChecks = stateCompat?.checks || {};
  const liveRehearsalOk =
    liveChecks.live_taskgroup_rehearsal_verified === true &&
    liveChecks.live_receipt_rehearsal_verified === true &&
    liveChecks.live_memory_surface_verified === true &&
    liveChecks.live_runtime_receipt_verified === true &&
    liveChecks.live_assimilation_contract_verified === true;
  const rcSteps = Array.isArray(rcRehearsal?.steps) ? rcRehearsal.steps : [];
  const passedRcStepIds = new Set(
    rcSteps.filter((row: any) => row?.ok === true).map((row: any) => String(row?.gate_id || '')),
  );
  const activeRcCycle = parseBool(process.env.INFRING_RELEASE_RC_REHEARSAL_ACTIVE, false);
  const rcRequiredStepsOk =
    requiredRcStepIds.length > 0 &&
    requiredRcStepIds.every((gateId) => passedRcStepIds.has(String(gateId)));
  const ipcSuccessRate = safeNumber(scorecard?.thresholds?.ipc_success_rate, Number.NaN);
  const receiptCompletenessRate = safeNumber(
    scorecard?.thresholds?.receipt_completeness_rate,
    Number.NaN,
  );
  const supportedCommandLatencyMs = safeNumber(
    scorecard?.thresholds?.max_command_latency_ms,
    safeNumber(scorecard?.thresholds?.supported_command_latency_ms, Number.NaN),
  );
  const observedRtoMinutes = safeNumber(scorecard?.thresholds?.observed_rto_minutes, Number.NaN);
  const observedRpoHours = safeNumber(scorecard?.thresholds?.observed_rpo_hours, Number.NaN);
  const directThresholdChecks: Check[] = [
    {
      id: 'release_metric:ipc_success_rate',
      ok:
        ipcSuccessRate >= safeNumber(thresholds.ipc_success_rate_min, 0.95) &&
        scorecardGates.get('ipc_success_rate_threshold')?.ok === true,
      detail:
        `value=${metricText(ipcSuccessRate)};min=${metricText(thresholds.ipc_success_rate_min)};` +
        `scorecard_gate=${scorecardGates.get('ipc_success_rate_threshold')?.ok === true}`,
    },
    {
      id: 'release_metric:receipt_completeness_rate',
      ok:
        receiptCompletenessRate >= safeNumber(thresholds.receipt_completeness_rate_min, 1) &&
        scorecardGates.get('receipt_completeness_threshold')?.ok === true,
      detail:
        `value=${metricText(receiptCompletenessRate)};min=${metricText(thresholds.receipt_completeness_rate_min)};` +
        `scorecard_gate=${scorecardGates.get('receipt_completeness_threshold')?.ok === true}`,
    },
    {
      id: 'release_metric:supported_command_latency_ms',
      ok:
        supportedCommandLatencyMs <= safeNumber(thresholds.supported_command_latency_ms_max, 2500) &&
        scorecardGates.get('supported_command_latency_threshold')?.ok === true,
      detail:
        `value=${metricText(supportedCommandLatencyMs)};max=${metricText(thresholds.supported_command_latency_ms_max)};` +
        `scorecard_gate=${scorecardGates.get('supported_command_latency_threshold')?.ok === true}`,
    },
    {
      id: 'release_metric:recovery_rto_minutes',
      ok:
        observedRtoMinutes <= safeNumber(thresholds.recovery_rto_minutes_max, 30) &&
        scorecardGates.get('recovery_rto_threshold')?.ok === true,
      detail:
        `value=${metricText(observedRtoMinutes)};max=${metricText(thresholds.recovery_rto_minutes_max)};` +
        `scorecard_gate=${scorecardGates.get('recovery_rto_threshold')?.ok === true}`,
    },
    {
      id: 'release_metric:recovery_rpo_hours',
      ok:
        observedRpoHours <= safeNumber(thresholds.recovery_rpo_hours_max, 24) &&
        scorecardGates.get('recovery_rpo_threshold')?.ok === true,
      detail:
        `value=${metricText(observedRpoHours)};max=${metricText(thresholds.recovery_rpo_hours_max)};` +
        `scorecard_gate=${scorecardGates.get('recovery_rpo_threshold')?.ok === true}`,
    },
  ];
  const clientBoundaryOk = clientBoundary?.summary?.pass === true || clientBoundary?.ok === true;
  return [
    {
      id: 'release_scorecard_numeric_thresholds',
      ok: scorecard?.ok === true,
      detail: scorecard?.ok === true ? 'enforced' : 'missing_or_failed',
    },
    ...directThresholdChecks,
    {
      id: 'production_topology_supported',
      ok:
        topology?.ok === true &&
        topology?.supported_production_topology === true &&
        Array.isArray(topology?.degraded_flags) &&
        topology.degraded_flags.length === 0,
      detail: `support_level=${String(topology?.support_level || 'unknown')}`,
    },
    {
      id: 'production_topology_degraded_flags_clear',
      ok: Array.isArray(topology?.degraded_flags) && topology.degraded_flags.length === 0,
      detail: `flags=${Array.isArray(topology?.degraded_flags) ? topology.degraded_flags.join(',') || 'none' : 'missing'}`,
    },
    {
      id: 'stateful_upgrade_live_rehearsal',
      ok: stateCompat?.ok === true && liveRehearsalOk,
      detail: `live_rehearsal=${liveRehearsalOk}`,
    },
    {
      id: 'release_candidate_rehearsal_completed',
      ok: !finalStage || activeRcCycle || (rcRehearsal?.ok === true && rcRequiredStepsOk),
      detail:
        !finalStage
          ? 'stage=prebundle;rc_rehearsal_not_required'
          : activeRcCycle
          ? 'current_rc_cycle_active'
          : `required_steps=${requiredRcStepIds.length};present=${rcRequiredStepsOk};` +
            `failed=${safeNumber(rcRehearsal?.summary?.failed_count, -1)}`,
    },
    {
      id: 'client_authority_regression_guard',
      ok:
        clientBoundaryOk &&
        (!finalStage || activeRcCycle || passedRcStepIds.has(clientAuthorityGateId)),
      detail:
        !finalStage
          ? `stage=prebundle;violations=${safeNumber(clientBoundary?.summary?.violation_count, -1)}`
          : activeRcCycle
          ? `current_rc_cycle_active;violations=${safeNumber(clientBoundary?.summary?.violation_count, -1)}`
          : `rc_step=${clientAuthorityGateId};violations=${safeNumber(clientBoundary?.summary?.violation_count, -1)}`,
    },
    {
      id: 'support_bundle_incident_truth_package',
      ok: !finalStage || supportBundle?.incident_truth_package?.ready === true,
      detail: !finalStage
        ? 'stage=prebundle;final_bundle_truth_not_required'
        : `failed_checks=${Array.isArray(supportBundle?.incident_truth_package?.failed_checks) ? supportBundle.incident_truth_package.failed_checks.length : 'missing'}`,
    },
  ];
}

function buildReport(args: Args) {
  const checks: Check[] = [];
  if (!fs.existsSync(POLICY_PATH)) {
    checks.push({
      id: 'policy_file',
      ok: false,
      detail: 'client/runtime/config/production_readiness_closure_policy.json missing',
    });
  }
  const policy = readJson<Policy>(POLICY_PATH, {});
  checks.push(...checkRequiredFiles(policy.required_files || []));
  checks.push(...checkPackageScripts(policy.required_package_scripts || []));
  checks.push(...checkWorkflowMarkers(policy.required_ci_invocations || []));
  checks.push(
    ...checkTextMarkers(
      path.join(ROOT, 'verify.sh'),
      policy.required_verify_invocations || [],
      'verify_invocation',
    ),
  );
  checks.push(...checkVerifyProfileGateIds(policy.required_verify_profile_gate_ids || {}));
  checks.push(
    ...checkTextMarkers(
      path.join(ROOT, 'README.md'),
      policy.required_readme_markers || [],
      'readme_marker',
    ),
  );
  if (args.runSmoke) checks.push(...runSmokeScripts(policy.smoke_scripts || []));
  checks.push(...checkReleaseEvidence(policy, args));

  const failed = checks.filter((row) => !row.ok);
  return {
    type: 'production_readiness_closure_gate',
    generated_at: new Date().toISOString(),
    strict: args.strict,
    run_smoke: args.runSmoke,
    stage: args.stage,
    summary: {
      check_count: checks.length,
      failed_count: failed.length,
      pass: failed.length === 0,
    },
    failed_ids: failed.map((row) => row.id),
    checks,
  };
}

export function run(rawArgs: Args | string[]): number {
  const args = Array.isArray(rawArgs) ? parseArgs(rawArgs) : rawArgs;
  const report = buildReport(args);
  const outPath = path.resolve(ROOT, args.out);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  if (args.strict && report.summary.failed_count > 0) return 1;
  return 0;
}

if (require.main === module) {
  process.exit(run(parseArgs(process.argv.slice(2))));
}

module.exports = {
  buildReport,
  parseArgs,
  run,
};
