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
  release_verdict?: {
    required_gate_artifacts?: Record<string, string>;
  };
  standing_regression_guards?: {
    client_authority_gate_id?: string;
  };
  release_evidence_flow?: {
    scorecard_stage?: 'prebundle' | 'final' | string;
    final_closure_stage?: 'prebundle' | 'final' | string;
    support_bundle_precedes_final_closure?: boolean;
  };
};

const ROOT = process.cwd();
const POLICY_PATH = path.join(ROOT, 'client/runtime/config/production_readiness_closure_policy.json');
const VERIFY_PROFILES_PATH = path.join(ROOT, 'tests/tooling/config/verify_profiles.json');
const GATE_REGISTRY_PATH = 'tests/tooling/config/tooling_gate_registry.json';
const RELEASE_GATES_PATH = path.join(ROOT, 'tests/tooling/config/release_gates.yaml');
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
const RELEASE_PROOF_PACK_PATH = path.join(
  ROOT,
  'core/local/artifacts/release_proof_pack_current.json',
);
const CLIENT_BOUNDARY_ARTIFACT_PATH = path.join(
  ROOT,
  'core/local/artifacts/client_layer_boundary_audit_current.json',
);
const RELEASE_GATE_TELEMETRY_PROFILES = ['rich', 'pure', 'tiny-max'] as const;
const RELEASE_GATE_TELEMETRY_KEYS = [
  'workflow_unexpected_state_loop_max',
  'automatic_tool_trigger_events_max',
  'file_tool_route_misdirection_max',
] as const;
const RELEASE_PROOF_PACK_REQUIRED_ARTIFACT_PATHS = [
  'core/local/artifacts/layer2_lane_parity_guard_current.json',
  'core/local/artifacts/layer2_receipt_replay_current.json',
  'core/local/artifacts/runtime_trusted_core_report_current.json',
] as const;
const RELEASE_POLICY_REQUIRED_GATE_IDS = [
  'ops:layer2:parity:guard',
  'ops:layer2:receipt:replay',
  'ops:trusted-core:report',
  'ops:release:proof-pack',
] as const;
const RELEASE_POLICY_REQUIRED_GATE_ARTIFACT_PATHS: Record<string, string> = {
  'ops:layer2:parity:guard': 'core/local/artifacts/layer2_lane_parity_guard_current.json',
  'ops:layer2:receipt:replay': 'core/local/artifacts/layer2_receipt_replay_current.json',
  'ops:trusted-core:report': 'core/local/artifacts/runtime_trusted_core_report_current.json',
  'ops:release:proof-pack': 'core/local/artifacts/release_proof_pack_current.json',
};
const RELEASE_PROOF_PACK_REQUIRED_CATEGORIES = [
  'runtime_proof',
  'adapter_and_orchestration',
  'release_governance',
  'workload_and_quality',
] as const;

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

function reEscape(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function isSha256Hex(value: unknown): boolean {
  const normalized = String(value || '').trim();
  return /^[a-f0-9]{64}$/i.test(normalized);
}

function isGitRevisionToken(value: unknown): boolean {
  const normalized = String(value || '').trim();
  return /^[a-f0-9]{7,40}$/i.test(normalized);
}

function isPathWithin(rootPath: string, candidatePath: string): boolean {
  const root = String(rootPath || '').trim();
  const candidate = String(candidatePath || '').trim();
  if (!root || !candidate) return false;
  const normalizedRoot = path.resolve(root);
  const normalizedCandidate = path.resolve(candidate);
  return (
    normalizedCandidate === normalizedRoot ||
    normalizedCandidate.startsWith(`${normalizedRoot}${path.sep}`)
  );
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

function checkReleaseGateTelemetryThresholds(): Check[] {
  const checks: Check[] = [];
  if (!fs.existsSync(RELEASE_GATES_PATH)) {
    return [
      {
        id: 'release_gates_file',
        ok: false,
        detail: 'tests/tooling/config/release_gates.yaml missing',
      },
    ];
  }
  const source = fs.readFileSync(RELEASE_GATES_PATH, 'utf8');
  const versionMatch = source.match(/^version:\s*([0-9]+(?:\.[0-9]+)?)\s*$/m);
  checks.push({
    id: 'release_gates_version_present',
    ok: !!versionMatch,
    detail: versionMatch ? `value=${versionMatch[1]}` : 'missing',
  });
  checks.push({
    id: 'release_gates_version_is_1',
    ok: !!versionMatch && Number(versionMatch[1]) === 1,
    detail: versionMatch ? `value=${versionMatch[1]};required=1` : 'missing',
  });
  for (const profile of RELEASE_GATE_TELEMETRY_PROFILES) {
    const sectionMatch = source.match(
      new RegExp(
        `^\\s{2}${reEscape(profile)}:\\n([\\s\\S]*?)(?=^\\s{2}[a-z0-9\\-]+:\\n|\\Z)`,
        'm',
      ),
    );
    checks.push({
      id: `release_gates_profile:${profile}`,
      ok: !!sectionMatch,
      detail: sectionMatch ? 'present' : 'missing',
    });
    if (!sectionMatch) continue;
    const section = sectionMatch[1];
    const syntheticRequiredMatch = section.match(
      /^\s{6}synthetic_required:\s*([0-9]+(?:\.[0-9]+)?)\s*$/m,
    );
    const empiricalRequiredMatch = section.match(
      /^\s{6}empirical_required:\s*([0-9]+(?:\.[0-9]+)?)\s*$/m,
    );
    const empiricalMinSamplesMatch = section.match(
      /^\s{6}empirical_min_sample_points:\s*([0-9]+(?:\.[0-9]+)?)\s*$/m,
    );
    checks.push({
      id: `release_gates_proof_track_synthetic_required:${profile}`,
      ok: !!syntheticRequiredMatch && Number(syntheticRequiredMatch[1]) === 1,
      detail: syntheticRequiredMatch ? `value=${syntheticRequiredMatch[1]};required=1` : 'missing',
    });
    checks.push({
      id: `release_gates_proof_track_empirical_required:${profile}`,
      ok: !!empiricalRequiredMatch && Number(empiricalRequiredMatch[1]) === 1,
      detail: empiricalRequiredMatch ? `value=${empiricalRequiredMatch[1]};required=1` : 'missing',
    });
    checks.push({
      id: `release_gates_proof_track_empirical_min_sample_points_positive:${profile}`,
      ok: !!empiricalMinSamplesMatch && Number(empiricalMinSamplesMatch[1]) > 0,
      detail: empiricalMinSamplesMatch
        ? `value=${empiricalMinSamplesMatch[1]};required>0`
        : 'missing',
    });
    const baselinePassRatioMatch = section.match(
      /^\s{6}baseline_pass_ratio_min:\s*([0-9]+(?:\.[0-9]+)?)\s*$/m,
    );
    const failClosedRatioMatch = section.match(
      /^\s{6}fail_closed_ratio_min:\s*([0-9]+(?:\.[0-9]+)?)\s*$/m,
    );
    const graduationRatioMatch = section.match(
      /^\s{6}graduation_ratio_min:\s*([0-9]+(?:\.[0-9]+)?)\s*$/m,
    );
    checks.push({
      id: `release_gates_gateway_chaos_baseline_ratio_min:${profile}`,
      ok: !!baselinePassRatioMatch && Number(baselinePassRatioMatch[1]) >= 1,
      detail: baselinePassRatioMatch ? `value=${baselinePassRatioMatch[1]};required>=1` : 'missing',
    });
    checks.push({
      id: `release_gates_gateway_chaos_fail_closed_ratio_min:${profile}`,
      ok: !!failClosedRatioMatch && Number(failClosedRatioMatch[1]) >= 1,
      detail: failClosedRatioMatch ? `value=${failClosedRatioMatch[1]};required>=1` : 'missing',
    });
    checks.push({
      id: `release_gates_gateway_chaos_graduation_ratio_min:${profile}`,
      ok: !!graduationRatioMatch && Number(graduationRatioMatch[1]) >= 1,
      detail: graduationRatioMatch ? `value=${graduationRatioMatch[1]};required>=1` : 'missing',
    });
    for (const key of RELEASE_GATE_TELEMETRY_KEYS) {
      const keyMatch = section.match(
        new RegExp(`^\\s{6}${reEscape(key)}:\\s*([0-9]+(?:\\.[0-9]+)?)\\s*$`, 'm'),
      );
      const numericValue = Number(keyMatch?.[1] ?? Number.NaN);
      checks.push({
        id: `release_gates_quality_telemetry_key:${profile}:${key}`,
        ok: !!keyMatch,
        detail: keyMatch ? `value=${keyMatch[1]}` : 'missing',
      });
      checks.push({
        id: `release_gates_quality_telemetry_value_zero:${profile}:${key}`,
        ok: !!keyMatch && Number.isFinite(numericValue) && numericValue === 0,
        detail: keyMatch ? `value=${keyMatch[1]};required=0` : 'missing',
      });
    }
  }
  return checks;
}

function checkReleaseEvidence(policy: Policy, args: Args): Check[] {
  const topology = readJson<any>(args.topologyPath, {});
  const stateCompat = readJson<any>(args.stateCompatPath, {});
  const supportBundle = readJson<any>(args.supportBundlePath, {});
  const scorecard = readJson<any>(args.scorecardPath, {});
  const rcRehearsal = readJson<any>(args.rcRehearsalPath, {});
  const clientBoundary = readJson<any>(args.clientBoundaryPath, {});
  const releaseProofPack = readJson<any>(RELEASE_PROOF_PACK_PATH, {});
  const releaseProofPackSummary =
    releaseProofPack?.summary && typeof releaseProofPack.summary === 'object'
      ? releaseProofPack.summary
      : {};
  const releaseProofPackArtifacts = Array.isArray(releaseProofPack?.artifacts)
    ? releaseProofPack.artifacts
    : [];
  const releaseProofPackArtifactRows = new Map<string, any>(
    releaseProofPackArtifacts
      .filter((row: any) => row && typeof row.path === 'string')
      .map((row: any) => [String(row.path), row]),
  );
  const releaseProofPackGeneratedAtMs = Date.parse(
    String(releaseProofPack?.generated_at || ''),
  );
  const releaseProofPackRevision = String(releaseProofPack?.revision || '')
    .trim()
    .slice(0, 120);
  const releaseProofPackCategoryCompletenessMin =
    releaseProofPack?.category_completeness_min &&
    typeof releaseProofPack.category_completeness_min === 'object'
      ? releaseProofPack.category_completeness_min
      : {};
  const releaseProofPackCategories = new Set<string>(
    releaseProofPackArtifacts
      .map((row: any) => String(row?.category || '').trim())
      .filter((row: string) => row.length > 0),
  );
  const releaseProofPackRequiredArtifactCount = releaseProofPackArtifacts.filter(
    (row: any) => row?.required === true,
  ).length;
  const releaseProofPackPackRoot = String(releaseProofPack?.pack_root || '').trim();
  const releaseProofPackPackRootIsAbsolute =
    releaseProofPackPackRoot.length > 0 && path.isAbsolute(releaseProofPackPackRoot);
  const releaseProofPackCanonicalRoot = path.resolve(ROOT, 'releases/proof-packs');
  const releaseProofPackSourceManifestPath = String(
    releaseProofPack?.source_manifest_path || '',
  ).trim();
  const releaseProofPackArtifactPathRows = releaseProofPackArtifacts
    .map((row: any) => String(row?.path || '').trim())
    .filter((artifactPath: string) => artifactPath.length > 0);
  const duplicateReleaseProofPackArtifactPaths = Array.from(
    releaseProofPackArtifactPathRows.reduce((acc, artifactPath) => {
      acc.set(artifactPath, (acc.get(artifactPath) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([artifactPath, count]) => `${artifactPath}:${count}`);
  const nonRelativeReleaseProofPackArtifactPaths = releaseProofPackArtifactPathRows
    .filter((artifactPath) => path.isAbsolute(artifactPath) || artifactPath.includes('\\'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const nonCoreReleaseProofPackArtifactPaths = releaseProofPackArtifactPathRows
    .filter((artifactPath) => !artifactPath.startsWith('core/local/artifacts/'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const releaseProofPackArtifactRowsMissingPath = releaseProofPackArtifacts
    .filter((row: any) => String(row?.path || '').trim().length === 0)
    .map((row: any) => JSON.stringify(row).slice(0, 180))
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackArtifactRowsPathWhitespace = releaseProofPackArtifactPathRows
    .filter((artifactPath) => /\s/.test(artifactPath))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const releaseProofPackArtifactRowsMissingCategory = releaseProofPackArtifacts
    .filter((row: any) => String(row?.category || '').trim().length === 0)
    .map((row: any) => String(row?.path || 'unknown'))
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackArtifactRowsNonCanonicalCategory = releaseProofPackArtifacts
    .filter((row: any) => String(row?.category || '').trim().length > 0)
    .filter((row: any) => !/^[a-z0-9_]+$/.test(String(row?.category || '').trim()))
    .map((row: any) => `${String(row?.path || 'unknown')}:${String(row?.category || '').trim()}`)
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackArtifactRowsUnknownCategory = releaseProofPackArtifacts
    .filter((row: any) => String(row?.category || '').trim().length > 0)
    .filter(
      (row: any) => !RELEASE_PROOF_PACK_REQUIRED_CATEGORIES.includes(
        String(row?.category || '').trim() as (typeof RELEASE_PROOF_PACK_REQUIRED_CATEGORIES)[number],
      ),
    )
    .map((row: any) => `${String(row?.path || 'unknown')}:${String(row?.category || '').trim()}`)
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackArtifactRowsRequiredFlagNonBoolean = releaseProofPackArtifacts
    .filter((row: any) => typeof row?.required !== 'boolean')
    .map((row: any) => `${String(row?.path || 'unknown')}:${typeof row?.required}`)
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackArtifactRowsExistsFlagNonBoolean = releaseProofPackArtifacts
    .filter((row: any) => typeof row?.exists !== 'boolean')
    .map((row: any) => `${String(row?.path || 'unknown')}:${typeof row?.exists}`)
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackArtifactRowsMissingSource = releaseProofPackArtifacts
    .filter((row: any) => String(row?.source || '').trim().length === 0)
    .map((row: any) => String(row?.path || 'unknown'))
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackArtifactRowsMissingDestination = releaseProofPackArtifacts
    .filter((row: any) => String(row?.destination || '').trim().length === 0)
    .map((row: any) => String(row?.path || 'unknown'))
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackArtifactRowsOutsidePackRoot = releaseProofPackArtifacts
    .filter((row: any) => String(row?.destination || '').trim().length > 0)
    .filter((row: any) => !isPathWithin(releaseProofPackPackRoot, String(row?.destination || '')))
    .map((row: any) => `${String(row?.path || 'unknown')}:${String(row?.destination || '')}`)
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackExistingArtifactRows = releaseProofPackArtifacts
    .filter((row: any) => row?.exists === true);
  const releaseProofPackExistingArtifactRowsMissingChecksum = releaseProofPackExistingArtifactRows
    .filter((row: any) => String(row?.checksum || '').trim().length === 0)
    .map((row: any) => String(row?.path || 'unknown'))
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackExistingArtifactRowsInvalidChecksum = releaseProofPackExistingArtifactRows
    .filter((row: any) => String(row?.checksum || '').trim().length > 0)
    .filter((row: any) => !isSha256Hex(row?.checksum))
    .map((row: any) => `${String(row?.path || 'unknown')}:${String(row?.checksum || '')}`)
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackExistingArtifactRowsNonPositiveSize = releaseProofPackExistingArtifactRows
    .filter((row: any) => !Number.isFinite(Number(row?.size_bytes)) || Number(row?.size_bytes) <= 0)
    .map((row: any) => `${String(row?.path || 'unknown')}:${metricText(row?.size_bytes)}`)
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackExistingArtifactRowsNonAbsoluteSource = releaseProofPackExistingArtifactRows
    .filter((row: any) => !path.isAbsolute(String(row?.source || '').trim()))
    .map((row: any) => `${String(row?.path || 'unknown')}:${String(row?.source || '')}`)
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackExistingArtifactRowsNonAbsoluteDestination = releaseProofPackExistingArtifactRows
    .filter((row: any) => !path.isAbsolute(String(row?.destination || '').trim()))
    .map((row: any) => `${String(row?.path || 'unknown')}:${String(row?.destination || '')}`)
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackRequiredArtifactRows = releaseProofPackArtifacts.filter(
    (row: any) => row?.required === true,
  );
  const releaseProofPackRequiredArtifactRowsMissingChecksum = releaseProofPackRequiredArtifactRows
    .filter((row: any) => String(row?.checksum || '').trim().length === 0)
    .map((row: any) => String(row?.path || 'unknown'))
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackRequiredArtifactRowsInvalidChecksum = releaseProofPackRequiredArtifactRows
    .filter((row: any) => String(row?.checksum || '').trim().length > 0)
    .filter((row: any) => !isSha256Hex(row?.checksum))
    .map((row: any) => `${String(row?.path || 'unknown')}:${String(row?.checksum || '')}`)
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackRequiredArtifactRowsWithNonPositiveSize = releaseProofPackRequiredArtifactRows
    .filter((row: any) => !Number.isFinite(Number(row?.size_bytes)) || Number(row?.size_bytes) <= 0)
    .map((row: any) => `${String(row?.path || 'unknown')}:${metricText(row?.size_bytes)}`)
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackRequiredArtifactRowsOutsideRepo = releaseProofPackRequiredArtifactRows
    .filter((row: any) => String(row?.source || '').trim().length > 0)
    .filter((row: any) => !isPathWithin(ROOT, String(row?.source || '')))
    .map((row: any) => `${String(row?.path || 'unknown')}:${String(row?.source || '')}`)
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const releaseProofPackRequiredArtifactRowsOutsidePackRoot = releaseProofPackRequiredArtifactRows
    .filter((row: any) => String(row?.destination || '').trim().length > 0)
    .filter((row: any) => !isPathWithin(releaseProofPackPackRoot, String(row?.destination || '')))
    .map((row: any) => `${String(row?.path || 'unknown')}:${String(row?.destination || '')}`)
    .sort((a: string, b: string) => a.localeCompare(b, 'en'));
  const supportBundleReleaseVerdict =
    supportBundle?.closure_evidence?.release_verdict || supportBundle?.release_verdict || null;
  const supportBundleReleaseProofPack =
    supportBundle?.closure_evidence?.release_proof_pack || null;
  const supportBundleReleaseProofPackSummary =
    supportBundleReleaseProofPack?.summary &&
    typeof supportBundleReleaseProofPack.summary === 'object'
      ? supportBundleReleaseProofPack.summary
      : {};
  const supportBundleReleaseProofPackRevision = String(
    supportBundleReleaseProofPack?.revision ||
    supportBundleReleaseProofPackSummary?.revision ||
    '',
  )
    .trim()
    .slice(0, 120);
  const supportBundleReleaseProofPackGeneratedAtMs = Date.parse(
    String(
      supportBundleReleaseProofPack?.generated_at ||
      supportBundleReleaseProofPackSummary?.generated_at ||
      '',
    ),
  );
  const supportBundleReleaseVerdictRevision = String(
    supportBundleReleaseVerdict?.revision ||
    supportBundleReleaseVerdict?.git_revision ||
    supportBundleReleaseVerdict?.git_sha ||
    '',
  )
    .trim()
    .slice(0, 120);
  const scorecardRevision = String(
    scorecard?.revision ||
    scorecard?.git?.revision ||
    scorecard?.git_revision ||
    scorecard?.git_sha ||
    '',
  )
    .trim()
    .slice(0, 120);
  const supportBundleReleaseVerdictChecks = Array.isArray(supportBundleReleaseVerdict?.checks)
    ? supportBundleReleaseVerdict.checks
    : [];
  const supportBundleReleaseVerdictCheckIds = supportBundleReleaseVerdictChecks
    .map((row: any) => String(row?.id || '').trim())
    .filter((gateId: string) => gateId.length > 0);
  const duplicateSupportBundleReleaseVerdictCheckIds = Array.from(
    supportBundleReleaseVerdictCheckIds.reduce((acc, gateId) => {
      acc.set(gateId, (acc.get(gateId) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([gateId, count]) => `${gateId}:${count}`);
  const invalidSupportBundleReleaseVerdictCheckIds = supportBundleReleaseVerdictCheckIds
    .filter((gateId) => !/^[a-z0-9:_-]+$/.test(gateId))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const supportBundleReleaseVerdictGeneratedAtMs = Date.parse(
    String(supportBundleReleaseVerdict?.generated_at || ''),
  );
  const supportBundleReleaseVerdictOk =
    supportBundleReleaseVerdict?.ok === true && supportBundleReleaseVerdictChecks.length > 0;
  const supportBundleProofPackRequiredMissingZero = supportBundleReleaseVerdictChecks.some(
    (row: any) => row?.id === 'release_proof_pack_required_missing_zero' && row?.ok === true,
  );
  const supportBundleProofPackCategoryThresholdsMet = supportBundleReleaseVerdictChecks.some(
    (row: any) => row?.id === 'release_proof_pack_category_thresholds_met' && row?.ok === true,
  );
  const thresholds = policy.numeric_thresholds || {};
  const releaseEvidenceFlow = policy.release_evidence_flow || {};
  const requiredScorecardStage = String(releaseEvidenceFlow.scorecard_stage || 'prebundle')
    .trim()
    .toLowerCase();
  const requiredFinalStage = String(releaseEvidenceFlow.final_closure_stage || 'final')
    .trim()
    .toLowerCase();
  const requireBundleBeforeFinal = releaseEvidenceFlow.support_bundle_precedes_final_closure !== false;
  const finalStage = args.stage === 'final';
  const scorecardStage = String(scorecard?.stage || '').trim().toLowerCase();
  const bundledScorecard = supportBundle?.closure_evidence?.release_scorecard || null;
  const bundledScorecardRevision = String(
    bundledScorecard?.revision ||
    bundledScorecard?.git?.revision ||
    bundledScorecard?.git_revision ||
    bundledScorecard?.git_sha ||
    '',
  )
    .trim()
    .slice(0, 120);
  const bundledScorecardStage = String(bundledScorecard?.stage || '').trim().toLowerCase();
  const bundledScorecardGeneratedAtMs = Date.parse(
    String(bundledScorecard?.generated_at || ''),
  );
  const scorecardGeneratedAtMs = Date.parse(
    String(scorecard?.generated_at || bundledScorecard?.generated_at || ''),
  );
  const supportBundleGeneratedAtMs = Date.parse(String(supportBundle?.generated_at || ''));
  const finalClosureFlowStage =
    args.stage === 'final' || args.stage === requiredFinalStage;
  const requiredRcStepIds = Array.isArray(policy.release_candidate_rehearsal?.required_step_gate_ids)
    ? policy.release_candidate_rehearsal?.required_step_gate_ids || []
    : [];
  const requiredGateArtifacts = policy.release_verdict?.required_gate_artifacts || {};
  const missingRequiredGateArtifactPolicyBindings = RELEASE_POLICY_REQUIRED_GATE_IDS
    .filter((gateId) => !requiredGateArtifacts[gateId])
    .map((gateId) => String(gateId));
  const mismatchedRequiredGateArtifactPolicyBindings = RELEASE_POLICY_REQUIRED_GATE_IDS
    .filter((gateId) => {
      const expected = String(RELEASE_POLICY_REQUIRED_GATE_ARTIFACT_PATHS[gateId] || '');
      const configured = String(requiredGateArtifacts[gateId] || '');
      if (!expected || !configured) return false;
      return configured !== expected;
    })
    .map((gateId) => `${gateId}:${String(requiredGateArtifacts[gateId] || '')}`);
  const missingRequiredGateArtifactFiles = RELEASE_POLICY_REQUIRED_GATE_IDS
    .filter((gateId) => {
      const artifactPath = String(requiredGateArtifacts[gateId] || '').trim();
      return artifactPath.length > 0 && !fs.existsSync(path.resolve(ROOT, artifactPath));
    })
    .map((gateId) => `${gateId}:${String(requiredGateArtifacts[gateId] || '')}`);
  const nonRelativeRequiredGateArtifactPaths = RELEASE_POLICY_REQUIRED_GATE_IDS
    .filter((gateId) => {
      const artifactPath = String(requiredGateArtifacts[gateId] || '').trim();
      if (!artifactPath) return false;
      return path.isAbsolute(artifactPath) || artifactPath.includes('\\');
    })
    .map((gateId) => `${gateId}:${String(requiredGateArtifacts[gateId] || '')}`);
  const nonCoreArtifactPrefixRequiredGateArtifactPaths = RELEASE_POLICY_REQUIRED_GATE_IDS
    .filter((gateId) => {
      const artifactPath = String(requiredGateArtifacts[gateId] || '').trim();
      if (!artifactPath) return false;
      return !artifactPath.startsWith('core/local/artifacts/');
    })
    .map((gateId) => `${gateId}:${String(requiredGateArtifacts[gateId] || '')}`);
  const duplicateRequiredGateArtifactPaths = Array.from(
    RELEASE_POLICY_REQUIRED_GATE_IDS.reduce((acc, gateId) => {
      const artifactPath = String(requiredGateArtifacts[gateId] || '').trim();
      if (!artifactPath) return acc;
      acc.set(artifactPath, (acc.get(artifactPath) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([artifactPath, count]) => `${artifactPath}:${count}`);
  const releasePolicyRequiredGateIdsAreUnique =
    new Set(RELEASE_POLICY_REQUIRED_GATE_IDS).size === RELEASE_POLICY_REQUIRED_GATE_IDS.length;
  const requiredGateArtifactBindingKeys = Object.keys(requiredGateArtifacts || {})
    .map((key) => String(key || '').trim())
    .filter((key) => key.length > 0);
  const requiredGateArtifactBindingKeysOutsideRequiredList = requiredGateArtifactBindingKeys
    .filter((key) => !RELEASE_POLICY_REQUIRED_GATE_IDS.includes(key))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const nonCanonicalRequiredGateArtifactBindingKeys = requiredGateArtifactBindingKeys
    .filter((key) => !/^ops:[a-z0-9:_-]+$/.test(key))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const requiredGateArtifactBindingRowsCount = RELEASE_POLICY_REQUIRED_GATE_IDS
    .filter((gateId) => String(requiredGateArtifacts[gateId] || '').trim().length > 0)
    .length;
  const requiredGateArtifactBindingKeysRaw = Object.keys(requiredGateArtifacts || {})
    .map((gateId) => String(gateId || ''));
  const emptyRequiredGateArtifactBindingKeys = requiredGateArtifactBindingKeysRaw
    .filter((gateId) => gateId.trim().length === 0);
  const requiredGateArtifactBindingPathsWithWhitespace = RELEASE_POLICY_REQUIRED_GATE_IDS
    .filter((gateId) => /\s/.test(String(requiredGateArtifacts[gateId] || '').trim()))
    .map((gateId) => `${gateId}:${String(requiredGateArtifacts[gateId] || '')}`);
  const requiredGateArtifactBindingPathsWithTraversal = RELEASE_POLICY_REQUIRED_GATE_IDS
    .filter((gateId) => String(requiredGateArtifacts[gateId] || '').trim().includes('..'))
    .map((gateId) => `${gateId}:${String(requiredGateArtifacts[gateId] || '')}`);
  const requiredGateArtifactBindingPathsNonJsonSuffix = RELEASE_POLICY_REQUIRED_GATE_IDS
    .filter((gateId) => String(requiredGateArtifacts[gateId] || '').trim().length > 0)
    .filter((gateId) => !String(requiredGateArtifacts[gateId] || '').trim().endsWith('.json'))
    .map((gateId) => `${gateId}:${String(requiredGateArtifacts[gateId] || '')}`);
  const requiredGateArtifactBindingPaths = RELEASE_POLICY_REQUIRED_GATE_IDS
    .map((gateId) => String(requiredGateArtifacts[gateId] || '').trim())
    .filter((artifactPath) => artifactPath.length > 0);
  const nonCanonicalRequiredGateArtifactBindingPaths = requiredGateArtifactBindingPaths
    .filter((artifactPath) => !/^core\/local\/artifacts\/[a-z0-9_./-]+_current\.json$/.test(artifactPath))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const dotPrefixedRequiredGateArtifactBindingPaths = requiredGateArtifactBindingPaths
    .filter((artifactPath) => artifactPath.startsWith('./') || artifactPath.startsWith('../'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const requiredGateArtifactBindingPathsOutsideRequiredPathContracts = requiredGateArtifactBindingPaths
    .filter(
      (artifactPath) =>
        !RELEASE_PROOF_PACK_REQUIRED_ARTIFACT_PATHS.includes(
          artifactPath as (typeof RELEASE_PROOF_PACK_REQUIRED_ARTIFACT_PATHS)[number],
        ),
    )
    .sort((a, b) => a.localeCompare(b, 'en'));
  const requiredPathContractsMissingFromPolicyBindings = RELEASE_PROOF_PACK_REQUIRED_ARTIFACT_PATHS
    .filter((artifactPath) => !requiredGateArtifactBindingPaths.includes(artifactPath))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const releaseProofPackRequiredArtifactPathsAreUnique =
    new Set(RELEASE_PROOF_PACK_REQUIRED_ARTIFACT_PATHS).size ===
    RELEASE_PROOF_PACK_REQUIRED_ARTIFACT_PATHS.length;
  const releaseProofPackRequiredArtifactPathsAreCanonical = RELEASE_PROOF_PACK_REQUIRED_ARTIFACT_PATHS
    .every((artifactPath) => String(artifactPath).startsWith('core/local/artifacts/'));
  const releaseProofPackRequiredCategoriesAreUnique =
    new Set(RELEASE_PROOF_PACK_REQUIRED_CATEGORIES).size ===
    RELEASE_PROOF_PACK_REQUIRED_CATEGORIES.length;
  const releaseProofPackCategoryMinimumKeys = Object.keys(
    releaseProofPackCategoryCompletenessMin || {},
  )
    .map((key) => String(key || '').trim())
    .filter((key) => key.length > 0);
  const releaseProofPackCategoryMinimumUnknownKeys = releaseProofPackCategoryMinimumKeys
    .filter(
      (category) =>
        !RELEASE_PROOF_PACK_REQUIRED_CATEGORIES.includes(
          category as (typeof RELEASE_PROOF_PACK_REQUIRED_CATEGORIES)[number],
        ),
    )
    .sort((a, b) => a.localeCompare(b, 'en'));
  const releaseProofPackCategoryMinimumNonFiniteOrNegative = releaseProofPackCategoryMinimumKeys
    .filter((category) => {
      const numeric = Number(releaseProofPackCategoryCompletenessMin[category]);
      return !Number.isFinite(numeric) || numeric < 0;
    })
    .map((category) => `${category}:${metricText(releaseProofPackCategoryCompletenessMin[category])}`)
    .sort((a, b) => a.localeCompare(b, 'en'));
  const releaseProofPackSummaryCountsAreNonNegativeIntegers =
    Number.isInteger(Number(releaseProofPackSummary.artifact_count)) &&
    Number(releaseProofPackSummary.artifact_count) >= 0 &&
    Number.isInteger(Number(releaseProofPackSummary.required_missing)) &&
    Number(releaseProofPackSummary.required_missing) >= 0 &&
    Number.isInteger(Number(releaseProofPackSummary.category_threshold_failure_count)) &&
    Number(releaseProofPackSummary.category_threshold_failure_count) >= 0;
  const releaseProofPackPackRootExistsOnDisk =
    releaseProofPackPackRoot.length > 0 &&
    releaseProofPackPackRootIsAbsolute &&
    fs.existsSync(releaseProofPackPackRoot);
  const releaseProofPackSourceManifestPathIsRelativeCanonical =
    releaseProofPackSourceManifestPath.length > 0 &&
    !path.isAbsolute(releaseProofPackSourceManifestPath) &&
    !releaseProofPackSourceManifestPath.includes('\\') &&
    !releaseProofPackSourceManifestPath.includes('..');
  const releaseProofPackSourceManifestExistsOnDisk =
    releaseProofPackSourceManifestPath.length > 0 &&
    fs.existsSync(path.resolve(ROOT, releaseProofPackSourceManifestPath));
  const supportBundleReleaseProofPackSummaryScalarsAreCanonical =
    Number.isInteger(Number(supportBundleReleaseProofPackSummary?.required_missing)) &&
    Number(supportBundleReleaseProofPackSummary?.required_missing) >= 0 &&
    Number.isInteger(Number(supportBundleReleaseProofPackSummary?.category_threshold_failure_count)) &&
    Number(supportBundleReleaseProofPackSummary?.category_threshold_failure_count) >= 0 &&
    typeof supportBundleReleaseProofPackSummary?.pass === 'boolean';
  const releaseGateTelemetryProfilesAreUnique =
    new Set(RELEASE_GATE_TELEMETRY_PROFILES).size === RELEASE_GATE_TELEMETRY_PROFILES.length;
  const releaseGateTelemetryKeysAreUnique =
    new Set(RELEASE_GATE_TELEMETRY_KEYS).size === RELEASE_GATE_TELEMETRY_KEYS.length;
  const releaseEvidenceFlowStageTokensAreCanonical =
    (requiredScorecardStage === 'prebundle' || requiredScorecardStage === 'final') &&
    (requiredFinalStage === 'prebundle' || requiredFinalStage === 'final');
  const releaseEvidenceFlowStageTokensAreDistinct =
    requiredScorecardStage !== requiredFinalStage;
  const scorecardGateRows = Array.isArray(scorecard?.gates) ? scorecard.gates : [];
  const scorecardGateIds = scorecardGateRows
    .map((row: any) => String(row?.id || '').trim());
  const duplicateScorecardGateIds = Array.from(
    scorecardGateIds.reduce((acc, gateId) => {
      if (!gateId) return acc;
      acc.set(gateId, (acc.get(gateId) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([gateId, count]) => `${gateId}:${count}`);
  const invalidScorecardGateIds = scorecardGateIds
    .filter((gateId) => gateId.length === 0 || /\s/.test(gateId))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const requiredRcStepIdsAreUnique = new Set(requiredRcStepIds).size === requiredRcStepIds.length;
  const requiredRcStepIdsWithWhitespace = requiredRcStepIds
    .map((gateId) => String(gateId || '').trim())
    .filter((gateId) => gateId.length === 0 || /\s/.test(gateId))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const clientAuthorityGateId = String(
    policy.standing_regression_guards?.client_authority_gate_id || 'audit:client-layer-boundary',
  );
  const scorecardGates = new Map<string, any>(
    (Array.isArray(scorecard?.gates) ? scorecard.gates : []).map((row: any) => [String(row?.id || ''), row]),
  );
  const requiredGateIdsMissingFromScorecard = RELEASE_POLICY_REQUIRED_GATE_IDS
    .filter((gateId) => !scorecardGates.has(String(gateId)))
    .map((gateId) => String(gateId));
  const requiredGateIdsFailingInScorecard = RELEASE_POLICY_REQUIRED_GATE_IDS
    .filter((gateId) => scorecardGates.has(String(gateId)))
    .filter((gateId) => scorecardGates.get(String(gateId))?.ok !== true)
    .map((gateId) => String(gateId));
  const liveChecks = stateCompat?.checks || {};
  const liveRehearsalOk =
    liveChecks.live_taskgroup_rehearsal_verified === true &&
    liveChecks.live_receipt_rehearsal_verified === true &&
    liveChecks.live_memory_surface_verified === true &&
    liveChecks.live_runtime_receipt_verified === true &&
    liveChecks.live_assimilation_contract_verified === true;
  const rcSteps = Array.isArray(rcRehearsal?.steps) ? rcRehearsal.steps : [];
  const rcStepIds = rcSteps.map((row: any) => String(row?.gate_id || '').trim());
  const duplicateRcStepIds = Array.from(
    rcStepIds.reduce((acc, gateId) => {
      if (!gateId) return acc;
      acc.set(gateId, (acc.get(gateId) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([gateId, count]) => `${gateId}:${count}`);
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
      id: 'release_evidence_flow_stage_matches_policy',
      ok: args.stage === requiredScorecardStage || args.stage === requiredFinalStage,
      detail:
        `invoked_stage=${args.stage};required_scorecard_stage=${requiredScorecardStage};` +
        `required_final_stage=${requiredFinalStage}`,
    },
    {
      id: 'release_evidence_flow_scorecard_stage_prebundle',
      ok:
        !finalClosureFlowStage ||
        (scorecardStage === requiredScorecardStage &&
          bundledScorecardStage === requiredScorecardStage),
      detail:
        !finalClosureFlowStage
          ? `stage=${args.stage};prebundle_scorecard_enforced_on_final_stage_only`
          : `scorecard_stage=${scorecardStage || 'missing'};` +
            `bundled_scorecard_stage=${bundledScorecardStage || 'missing'};` +
            `required=${requiredScorecardStage}`,
    },
    {
      id: 'release_evidence_flow_support_bundle_precedes_final_closure',
      ok:
        !finalClosureFlowStage ||
        !requireBundleBeforeFinal ||
        (fs.existsSync(args.supportBundlePath) &&
          Number.isFinite(scorecardGeneratedAtMs) &&
          Number.isFinite(supportBundleGeneratedAtMs) &&
          scorecardGeneratedAtMs <= supportBundleGeneratedAtMs),
      detail:
        !finalClosureFlowStage
          ? `stage=${args.stage};support_bundle_ordering_enforced_on_final_stage_only`
          : !requireBundleBeforeFinal
          ? 'policy_support_bundle_precedes_final_closure=false'
          : `scorecard_generated_at=${Number.isFinite(scorecardGeneratedAtMs) ? scorecardGeneratedAtMs : 'missing'};` +
            `support_bundle_generated_at=${Number.isFinite(supportBundleGeneratedAtMs) ? supportBundleGeneratedAtMs : 'missing'};` +
            `support_bundle_present=${fs.existsSync(args.supportBundlePath)}`,
    },
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
      id: 'release_candidate_required_step_ids_are_unique',
      ok: requiredRcStepIdsAreUnique,
      detail:
        `required_rc_step_ids=${requiredRcStepIds.length};` +
        `unique_required_rc_step_ids=${new Set(requiredRcStepIds).size}`,
    },
    {
      id: 'release_candidate_required_step_ids_are_nonempty_and_whitespace_free',
      ok: requiredRcStepIdsWithWhitespace.length === 0,
      detail:
        requiredRcStepIdsWithWhitespace.length === 0
          ? 'required RC step gate ids are non-empty and whitespace-free'
          : `invalid_required_rc_step_ids=${requiredRcStepIdsWithWhitespace.join(',')}`,
    },
    {
      id: 'release_candidate_step_rows_are_unique_by_gate_id',
      ok: !finalStage || duplicateRcStepIds.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;rc step-row uniqueness enforced on final stage only'
        : duplicateRcStepIds.length === 0
        ? 'RC rehearsal step gate ids are unique'
        : `duplicate_rc_step_gate_ids=${duplicateRcStepIds.join(',')}`,
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
    {
      id: 'release_evidence_flow_support_bundle_embeds_release_verdict',
      ok: !finalStage || supportBundleReleaseVerdictOk,
      detail: !finalStage
        ? 'stage=prebundle;final_release_verdict_embedding_not_required'
        : `release_verdict_ok=${supportBundleReleaseVerdict?.ok === true};` +
          `checks=${supportBundleReleaseVerdictChecks.length}`,
    },
    {
      id: 'release_evidence_flow_release_verdict_precedes_support_bundle',
      ok:
        !finalStage ||
        (Number.isFinite(supportBundleReleaseVerdictGeneratedAtMs) &&
          Number.isFinite(supportBundleGeneratedAtMs) &&
          supportBundleReleaseVerdictGeneratedAtMs <= supportBundleGeneratedAtMs),
      detail: !finalStage
        ? 'stage=prebundle;release_verdict_ordering_not_required'
        : `release_verdict_generated_at=${Number.isFinite(supportBundleReleaseVerdictGeneratedAtMs) ? supportBundleReleaseVerdictGeneratedAtMs : 'missing'};` +
          `support_bundle_generated_at=${Number.isFinite(supportBundleGeneratedAtMs) ? supportBundleGeneratedAtMs : 'missing'}`,
    },
    {
      id: 'release_evidence_flow_release_verdict_revision_matches_scorecard',
      ok:
        !finalStage ||
        (!!supportBundleReleaseVerdictRevision &&
          !!scorecardRevision &&
          supportBundleReleaseVerdictRevision === scorecardRevision),
      detail: !finalStage
        ? 'stage=prebundle;revision_alignment_not_required'
        : `release_verdict_revision=${supportBundleReleaseVerdictRevision || 'missing'};` +
          `scorecard_revision=${scorecardRevision || 'missing'}`,
    },
    {
      id: 'release_evidence_flow_release_verdict_check_ids_unique_and_canonical',
      ok:
        !finalStage ||
        (duplicateSupportBundleReleaseVerdictCheckIds.length === 0 &&
          invalidSupportBundleReleaseVerdictCheckIds.length === 0),
      detail: !finalStage
        ? 'stage=prebundle;release-verdict check-id canonicality enforced on final stage only'
        : duplicateSupportBundleReleaseVerdictCheckIds.length === 0 &&
          invalidSupportBundleReleaseVerdictCheckIds.length === 0
        ? `release_verdict_check_ids=${supportBundleReleaseVerdictCheckIds.length}`
        : `duplicate_release_verdict_check_ids=${duplicateSupportBundleReleaseVerdictCheckIds.join(',') || 'none'};` +
          `invalid_release_verdict_check_ids=${invalidSupportBundleReleaseVerdictCheckIds.join(',') || 'none'}`,
    },
    {
      id: 'release_evidence_flow_release_verdict_revision_matches_release_proof_pack',
      ok:
        !finalStage ||
        (!!supportBundleReleaseVerdictRevision &&
          !!releaseProofPackRevision &&
          supportBundleReleaseVerdictRevision === releaseProofPackRevision),
      detail: !finalStage
        ? 'stage=prebundle;release-verdict/proof-pack revision alignment enforced on final stage only'
        : `release_verdict_revision=${supportBundleReleaseVerdictRevision || 'missing'};` +
          `release_proof_pack_revision=${releaseProofPackRevision || 'missing'}`,
    },
    {
      id: 'release_evidence_flow_release_verdict_confirms_proof_pack_thresholds',
      ok:
        !finalStage ||
        (supportBundleProofPackRequiredMissingZero &&
          supportBundleProofPackCategoryThresholdsMet),
      detail: !finalStage
        ? 'stage=prebundle;proof_pack_verdict_confirmation_not_required'
        : `required_missing_zero=${supportBundleProofPackRequiredMissingZero};` +
          `category_thresholds_met=${supportBundleProofPackCategoryThresholdsMet}`,
    },
    {
      id: 'release_policy_required_gate_artifact_bindings_present',
      ok: missingRequiredGateArtifactPolicyBindings.length === 0,
      detail:
        missingRequiredGateArtifactPolicyBindings.length === 0
          ? 'release_verdict.required_gate_artifacts declares layer2 parity/replay/trusted-core/proof-pack bindings'
          : `missing_policy_gate_artifact_bindings=${missingRequiredGateArtifactPolicyBindings.join(',')}`,
    },
    {
      id: 'release_policy_required_gate_ids_are_unique',
      ok: releasePolicyRequiredGateIdsAreUnique,
      detail:
        `required_gate_ids=${RELEASE_POLICY_REQUIRED_GATE_IDS.length};` +
        `unique_required_gate_ids=${new Set(RELEASE_POLICY_REQUIRED_GATE_IDS).size}`,
    },
    {
      id: 'release_policy_required_gate_artifact_binding_keys_are_required',
      ok: requiredGateArtifactBindingKeysOutsideRequiredList.length === 0,
      detail:
        requiredGateArtifactBindingKeysOutsideRequiredList.length === 0
          ? 'release_verdict.required_gate_artifacts keys are constrained to policy-required gate ids'
          : `required_gate_artifact_binding_keys_outside_required_list=${requiredGateArtifactBindingKeysOutsideRequiredList.join(',')}`,
    },
    {
      id: 'release_policy_required_gate_artifact_binding_key_count_covers_required_ids',
      ok: requiredGateArtifactBindingKeys.length >= RELEASE_POLICY_REQUIRED_GATE_IDS.length,
      detail:
        `required_gate_artifact_binding_keys=${requiredGateArtifactBindingKeys.length};` +
        `required_gate_ids=${RELEASE_POLICY_REQUIRED_GATE_IDS.length}`,
    },
    {
      id: 'release_policy_required_gate_artifact_binding_keys_use_canonical_format',
      ok: nonCanonicalRequiredGateArtifactBindingKeys.length === 0,
      detail:
        nonCanonicalRequiredGateArtifactBindingKeys.length === 0
          ? 'release_verdict.required_gate_artifacts keys use canonical ops:<scope> gate-id format'
          : `noncanonical_required_gate_artifact_binding_keys=${nonCanonicalRequiredGateArtifactBindingKeys.join(',')}`,
    },
    {
      id: 'release_policy_required_gate_artifact_bindings_populated_for_required_ids',
      ok: requiredGateArtifactBindingRowsCount === RELEASE_POLICY_REQUIRED_GATE_IDS.length,
      detail:
        `populated_required_gate_artifact_bindings=${requiredGateArtifactBindingRowsCount};` +
        `required_gate_ids=${RELEASE_POLICY_REQUIRED_GATE_IDS.length}`,
    },
    {
      id: 'release_policy_required_gate_artifact_binding_keys_raw_nonempty',
      ok: emptyRequiredGateArtifactBindingKeys.length === 0,
      detail:
        emptyRequiredGateArtifactBindingKeys.length === 0
          ? 'release_verdict.required_gate_artifacts does not include empty-string keys'
          : `empty_required_gate_artifact_binding_keys=${emptyRequiredGateArtifactBindingKeys.length}`,
    },
    {
      id: 'release_policy_required_gate_artifact_binding_paths_whitespace_free',
      ok: requiredGateArtifactBindingPathsWithWhitespace.length === 0,
      detail:
        requiredGateArtifactBindingPathsWithWhitespace.length === 0
          ? 'release_verdict.required_gate_artifacts paths are whitespace-free'
          : `whitespace_policy_gate_artifact_paths=${requiredGateArtifactBindingPathsWithWhitespace.join(',')}`,
    },
    {
      id: 'release_policy_required_gate_artifact_binding_paths_traversal_free',
      ok: requiredGateArtifactBindingPathsWithTraversal.length === 0,
      detail:
        requiredGateArtifactBindingPathsWithTraversal.length === 0
          ? 'release_verdict.required_gate_artifacts paths are traversal-free'
          : `traversal_policy_gate_artifact_paths=${requiredGateArtifactBindingPathsWithTraversal.join(',')}`,
    },
    {
      id: 'release_policy_required_gate_artifact_binding_paths_json_suffix',
      ok: requiredGateArtifactBindingPathsNonJsonSuffix.length === 0,
      detail:
        requiredGateArtifactBindingPathsNonJsonSuffix.length === 0
          ? 'release_verdict.required_gate_artifacts paths end with .json'
          : `non_json_policy_gate_artifact_paths=${requiredGateArtifactBindingPathsNonJsonSuffix.join(',')}`,
    },
    {
      id: 'release_policy_required_gate_artifact_binding_paths_use_canonical_token_shape',
      ok: nonCanonicalRequiredGateArtifactBindingPaths.length === 0,
      detail:
        nonCanonicalRequiredGateArtifactBindingPaths.length === 0
          ? 'release_verdict.required_gate_artifacts paths use canonical core/local/artifacts/*_current.json token shape'
          : `noncanonical_policy_gate_artifact_paths=${nonCanonicalRequiredGateArtifactBindingPaths.join(',')}`,
    },
    {
      id: 'release_policy_required_gate_artifact_binding_paths_do_not_use_dot_prefixes',
      ok: dotPrefixedRequiredGateArtifactBindingPaths.length === 0,
      detail:
        dotPrefixedRequiredGateArtifactBindingPaths.length === 0
          ? 'release_verdict.required_gate_artifacts paths do not use ./ or ../ prefixes'
          : `dot_prefixed_policy_gate_artifact_paths=${dotPrefixedRequiredGateArtifactBindingPaths.join(',')}`,
    },
    {
      id: 'release_policy_required_gate_artifact_binding_paths_within_required_path_contract_set',
      ok: requiredGateArtifactBindingPathsOutsideRequiredPathContracts.length === 0,
      detail:
        requiredGateArtifactBindingPathsOutsideRequiredPathContracts.length === 0
          ? 'release_verdict.required_gate_artifacts paths are constrained to release proof-pack required path contracts'
          : `policy_gate_artifact_paths_outside_required_contracts=${requiredGateArtifactBindingPathsOutsideRequiredPathContracts.join(',')}`,
    },
    {
      id: 'release_policy_required_gate_artifact_bindings_cover_required_path_contract_set',
      ok: requiredPathContractsMissingFromPolicyBindings.length === 0,
      detail:
        requiredPathContractsMissingFromPolicyBindings.length === 0
          ? 'release_verdict.required_gate_artifacts covers all release proof-pack required path contracts'
          : `required_path_contracts_missing_from_policy_bindings=${requiredPathContractsMissingFromPolicyBindings.join(',')}`,
    },
    {
      id: 'release_policy_required_gate_artifact_bindings_match_expected',
      ok: mismatchedRequiredGateArtifactPolicyBindings.length === 0,
      detail:
        mismatchedRequiredGateArtifactPolicyBindings.length === 0
          ? 'release_verdict.required_gate_artifacts paths match canonical gate artifact contracts'
          : `mismatched_policy_gate_artifact_bindings=${mismatchedRequiredGateArtifactPolicyBindings.join(',')}`,
    },
    {
      id: 'release_policy_required_gate_artifact_files_exist',
      ok: !finalStage || missingRequiredGateArtifactFiles.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;policy gate-artifact file presence enforced on final stage only'
        : missingRequiredGateArtifactFiles.length === 0
        ? 'all required policy-declared gate artifact files are present'
        : `missing_policy_gate_artifact_files=${missingRequiredGateArtifactFiles.join(',')}`,
    },
    {
      id: 'release_policy_required_gate_artifact_paths_unique',
      ok: duplicateRequiredGateArtifactPaths.length === 0,
      detail:
        duplicateRequiredGateArtifactPaths.length === 0
          ? 'required gate-artifact paths are unique across policy-required release gates'
          : `duplicate_policy_gate_artifact_paths=${duplicateRequiredGateArtifactPaths.join(',')}`,
    },
    {
      id: 'release_policy_required_gate_artifact_paths_relative',
      ok: nonRelativeRequiredGateArtifactPaths.length === 0,
      detail:
        nonRelativeRequiredGateArtifactPaths.length === 0
          ? 'required gate-artifact paths are relative canonical repo paths'
          : `non_relative_policy_gate_artifact_paths=${nonRelativeRequiredGateArtifactPaths.join(',')}`,
    },
    {
      id: 'release_policy_required_gate_artifact_paths_under_core_local_artifacts',
      ok: nonCoreArtifactPrefixRequiredGateArtifactPaths.length === 0,
      detail:
        nonCoreArtifactPrefixRequiredGateArtifactPaths.length === 0
          ? 'required gate-artifact paths are rooted under core/local/artifacts/'
          : `non_core_artifact_prefix_policy_gate_paths=${nonCoreArtifactPrefixRequiredGateArtifactPaths.join(',')}`,
    },
    {
      id: 'release_policy_release_evidence_flow_stage_tokens_are_canonical',
      ok: releaseEvidenceFlowStageTokensAreCanonical,
      detail:
        `scorecard_stage=${requiredScorecardStage};` +
        `final_stage=${requiredFinalStage};allowed=prebundle|final`,
    },
    {
      id: 'release_policy_release_evidence_flow_stage_tokens_are_distinct',
      ok: releaseEvidenceFlowStageTokensAreDistinct,
      detail:
        `scorecard_stage=${requiredScorecardStage};` +
        `final_stage=${requiredFinalStage}`,
    },
    {
      id: 'release_policy_required_gate_ids_present_in_scorecard',
      ok: !finalStage || requiredGateIdsMissingFromScorecard.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;required scorecard gate-id presence enforced on final stage only'
        : requiredGateIdsMissingFromScorecard.length === 0
        ? 'all policy-required gate ids are present in release scorecard gates'
        : `missing_required_gate_ids_in_scorecard=${requiredGateIdsMissingFromScorecard.join(',')}`,
    },
    {
      id: 'release_policy_required_gate_ids_pass_in_scorecard',
      ok: !finalStage || requiredGateIdsFailingInScorecard.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;required scorecard gate-id pass status enforced on final stage only'
        : requiredGateIdsFailingInScorecard.length === 0
        ? 'all policy-required gate ids pass in release scorecard gates'
        : `failing_required_gate_ids_in_scorecard=${requiredGateIdsFailingInScorecard.join(',')}`,
    },
    {
      id: 'release_scorecard_gate_rows_present',
      ok: !finalStage || scorecardGateRows.length > 0,
      detail: !finalStage
        ? 'stage=prebundle;scorecard gate-row presence enforced on final stage only'
        : `scorecard_gate_rows=${scorecardGateRows.length}`,
    },
    {
      id: 'release_scorecard_gate_ids_are_unique',
      ok: !finalStage || duplicateScorecardGateIds.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;scorecard gate-id uniqueness enforced on final stage only'
        : duplicateScorecardGateIds.length === 0
        ? 'scorecard gate ids are unique'
        : `duplicate_scorecard_gate_ids=${duplicateScorecardGateIds.join(',')}`,
    },
    {
      id: 'release_scorecard_gate_ids_are_nonempty_and_whitespace_free',
      ok: !finalStage || invalidScorecardGateIds.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;scorecard gate-id token quality enforced on final stage only'
        : invalidScorecardGateIds.length === 0
        ? 'scorecard gate ids are non-empty and whitespace-free'
        : `invalid_scorecard_gate_ids=${invalidScorecardGateIds.join(',')}`,
    },
    {
      id: 'release_proof_pack_artifact_exists',
      ok: !finalStage || fs.existsSync(RELEASE_PROOF_PACK_PATH),
      detail: !finalStage
        ? 'stage=prebundle;proof_pack_artifact_presence_enforced_on_final_stage_only'
        : `path=${path.relative(ROOT, RELEASE_PROOF_PACK_PATH)};present=${fs.existsSync(RELEASE_PROOF_PACK_PATH)}`,
    },
    {
      id: 'release_proof_pack_pack_root_present',
      ok: !finalStage || releaseProofPackPackRoot.length > 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack pack_root presence enforced on final stage only'
        : `pack_root=${releaseProofPackPackRoot || 'missing'}`,
    },
    {
      id: 'release_proof_pack_pack_root_absolute',
      ok: !finalStage || releaseProofPackPackRootIsAbsolute,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack pack_root absolute-path contract enforced on final stage only'
        : `pack_root=${releaseProofPackPackRoot || 'missing'};is_absolute=${releaseProofPackPackRootIsAbsolute}`,
    },
    {
      id: 'release_proof_pack_pack_root_under_releases_proof_packs',
      ok:
        !finalStage ||
        (releaseProofPackPackRootIsAbsolute &&
          isPathWithin(releaseProofPackCanonicalRoot, releaseProofPackPackRoot)),
      detail: !finalStage
        ? 'stage=prebundle;proof-pack pack_root canonical-root contract enforced on final stage only'
        : `pack_root=${releaseProofPackPackRoot || 'missing'};canonical_root=${releaseProofPackCanonicalRoot}`,
    },
    {
      id: 'release_proof_pack_source_manifest_path_present',
      ok: !finalStage || releaseProofPackSourceManifestPath.length > 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack source manifest path presence enforced on final stage only'
        : `source_manifest_path=${releaseProofPackSourceManifestPath || 'missing'}`,
    },
    {
      id: 'release_proof_pack_source_manifest_path_canonical',
      ok:
        !finalStage ||
        releaseProofPackSourceManifestPath === 'tests/tooling/config/release_proof_pack_manifest.json',
      detail: !finalStage
        ? 'stage=prebundle;proof-pack source manifest canonical path enforced on final stage only'
        : `source_manifest_path=${releaseProofPackSourceManifestPath || 'missing'}`,
    },
    {
      id: 'release_proof_pack_pack_root_exists_on_disk',
      ok: !finalStage || releaseProofPackPackRootExistsOnDisk,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack pack_root filesystem existence enforced on final stage only'
        : `pack_root=${releaseProofPackPackRoot || 'missing'};exists=${releaseProofPackPackRootExistsOnDisk}`,
    },
    {
      id: 'release_proof_pack_source_manifest_path_relative_canonical',
      ok: !finalStage || releaseProofPackSourceManifestPathIsRelativeCanonical,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack source manifest relative path canonicality enforced on final stage only'
        : `source_manifest_path=${releaseProofPackSourceManifestPath || 'missing'};relative_canonical=${releaseProofPackSourceManifestPathIsRelativeCanonical}`,
    },
    {
      id: 'release_proof_pack_source_manifest_exists_on_disk',
      ok: !finalStage || releaseProofPackSourceManifestExistsOnDisk,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack source manifest filesystem existence enforced on final stage only'
        : `source_manifest_path=${releaseProofPackSourceManifestPath || 'missing'};exists=${releaseProofPackSourceManifestExistsOnDisk}`,
    },
    {
      id: 'release_proof_pack_artifact_rows_nonempty',
      ok: !finalStage || releaseProofPackArtifactPathRows.length > 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack artifact row presence enforced on final stage only'
        : `artifact_rows=${releaseProofPackArtifactPathRows.length}`,
    },
    {
      id: 'release_proof_pack_artifact_paths_unique',
      ok: !finalStage || duplicateReleaseProofPackArtifactPaths.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack artifact path uniqueness enforced on final stage only'
        : duplicateReleaseProofPackArtifactPaths.length === 0
        ? 'proof-pack artifact paths are unique'
        : `duplicate_proof_pack_artifact_paths=${duplicateReleaseProofPackArtifactPaths.join(',')}`,
    },
    {
      id: 'release_proof_pack_artifact_paths_relative',
      ok: !finalStage || nonRelativeReleaseProofPackArtifactPaths.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack artifact relative path contract enforced on final stage only'
        : nonRelativeReleaseProofPackArtifactPaths.length === 0
        ? 'proof-pack artifact paths are relative canonical repo paths'
        : `non_relative_proof_pack_artifact_paths=${nonRelativeReleaseProofPackArtifactPaths.join(',')}`,
    },
    {
      id: 'release_proof_pack_artifact_paths_under_core_local_artifacts',
      ok: !finalStage || nonCoreReleaseProofPackArtifactPaths.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack artifact prefix contract enforced on final stage only'
        : nonCoreReleaseProofPackArtifactPaths.length === 0
        ? 'proof-pack artifact paths are rooted under core/local/artifacts/'
        : `non_core_local_artifacts_paths=${nonCoreReleaseProofPackArtifactPaths.join(',')}`,
    },
    {
      id: 'release_proof_pack_artifact_rows_path_present',
      ok: !finalStage || releaseProofPackArtifactRowsMissingPath.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack artifact row path presence enforced on final stage only'
        : releaseProofPackArtifactRowsMissingPath.length === 0
        ? 'all proof-pack artifact rows declare path values'
        : `missing_proof_pack_artifact_row_paths=${releaseProofPackArtifactRowsMissingPath.join(',')}`,
    },
    {
      id: 'release_proof_pack_artifact_paths_whitespace_free',
      ok: !finalStage || releaseProofPackArtifactRowsPathWhitespace.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack artifact path whitespace contract enforced on final stage only'
        : releaseProofPackArtifactRowsPathWhitespace.length === 0
        ? 'proof-pack artifact paths are whitespace-free'
        : `whitespace_proof_pack_artifact_paths=${releaseProofPackArtifactRowsPathWhitespace.join(',')}`,
    },
    {
      id: 'release_proof_pack_artifact_rows_category_present',
      ok: !finalStage || releaseProofPackArtifactRowsMissingCategory.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack artifact row category presence enforced on final stage only'
        : releaseProofPackArtifactRowsMissingCategory.length === 0
        ? 'all proof-pack artifact rows declare category values'
        : `missing_proof_pack_artifact_row_categories=${releaseProofPackArtifactRowsMissingCategory.join(',')}`,
    },
    {
      id: 'release_proof_pack_artifact_rows_category_canonical',
      ok: !finalStage || releaseProofPackArtifactRowsNonCanonicalCategory.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack artifact category canonicality enforced on final stage only'
        : releaseProofPackArtifactRowsNonCanonicalCategory.length === 0
        ? 'all proof-pack artifact categories are canonical lowercase tokens'
        : `noncanonical_proof_pack_artifact_categories=${releaseProofPackArtifactRowsNonCanonicalCategory.join(',')}`,
    },
    {
      id: 'release_proof_pack_artifact_rows_category_known',
      ok: !finalStage || releaseProofPackArtifactRowsUnknownCategory.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack artifact category allowlist enforced on final stage only'
        : releaseProofPackArtifactRowsUnknownCategory.length === 0
        ? 'all proof-pack artifact categories are within required release category set'
        : `unknown_proof_pack_artifact_categories=${releaseProofPackArtifactRowsUnknownCategory.join(',')}`,
    },
    {
      id: 'release_proof_pack_artifact_rows_required_flag_boolean',
      ok: !finalStage || releaseProofPackArtifactRowsRequiredFlagNonBoolean.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack artifact required-flag type enforced on final stage only'
        : releaseProofPackArtifactRowsRequiredFlagNonBoolean.length === 0
        ? 'all proof-pack artifact rows use boolean required flags'
        : `non_boolean_proof_pack_required_flags=${releaseProofPackArtifactRowsRequiredFlagNonBoolean.join(',')}`,
    },
    {
      id: 'release_proof_pack_artifact_rows_exists_flag_boolean',
      ok: !finalStage || releaseProofPackArtifactRowsExistsFlagNonBoolean.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack artifact exists-flag type enforced on final stage only'
        : releaseProofPackArtifactRowsExistsFlagNonBoolean.length === 0
        ? 'all proof-pack artifact rows use boolean exists flags'
        : `non_boolean_proof_pack_exists_flags=${releaseProofPackArtifactRowsExistsFlagNonBoolean.join(',')}`,
    },
    {
      id: 'release_proof_pack_artifact_rows_source_present',
      ok: !finalStage || releaseProofPackArtifactRowsMissingSource.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack artifact source presence enforced on final stage only'
        : releaseProofPackArtifactRowsMissingSource.length === 0
        ? 'all proof-pack artifact rows declare source paths'
        : `missing_proof_pack_artifact_sources=${releaseProofPackArtifactRowsMissingSource.join(',')}`,
    },
    {
      id: 'release_proof_pack_artifact_rows_destination_present',
      ok: !finalStage || releaseProofPackArtifactRowsMissingDestination.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack artifact destination presence enforced on final stage only'
        : releaseProofPackArtifactRowsMissingDestination.length === 0
        ? 'all proof-pack artifact rows declare destination paths'
        : `missing_proof_pack_artifact_destinations=${releaseProofPackArtifactRowsMissingDestination.join(',')}`,
    },
    {
      id: 'release_proof_pack_artifact_rows_destinations_within_pack_root',
      ok: !finalStage || releaseProofPackArtifactRowsOutsidePackRoot.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack artifact destination pack-root boundary enforced on final stage only'
        : releaseProofPackArtifactRowsOutsidePackRoot.length === 0
        ? 'all proof-pack artifact destinations remain within proof-pack root'
        : `proof_pack_artifact_destinations_outside_pack_root=${releaseProofPackArtifactRowsOutsidePackRoot.join(',')}`,
    },
    {
      id: 'release_proof_pack_existing_artifact_rows_have_checksum',
      ok: !finalStage || releaseProofPackExistingArtifactRowsMissingChecksum.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;existing proof-pack artifact checksum presence enforced on final stage only'
        : releaseProofPackExistingArtifactRowsMissingChecksum.length === 0
        ? 'all existing proof-pack artifact rows declare checksums'
        : `existing_proof_pack_artifacts_missing_checksum=${releaseProofPackExistingArtifactRowsMissingChecksum.join(',')}`,
    },
    {
      id: 'release_proof_pack_existing_artifact_rows_checksum_is_sha256_hex',
      ok: !finalStage || releaseProofPackExistingArtifactRowsInvalidChecksum.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;existing proof-pack artifact checksum format enforced on final stage only'
        : releaseProofPackExistingArtifactRowsInvalidChecksum.length === 0
        ? 'all existing proof-pack artifact checksums are sha256 hex digests'
        : `existing_proof_pack_artifact_invalid_checksums=${releaseProofPackExistingArtifactRowsInvalidChecksum.join(',')}`,
    },
    {
      id: 'release_proof_pack_existing_artifact_rows_have_positive_size',
      ok: !finalStage || releaseProofPackExistingArtifactRowsNonPositiveSize.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;existing proof-pack artifact size contract enforced on final stage only'
        : releaseProofPackExistingArtifactRowsNonPositiveSize.length === 0
        ? 'all existing proof-pack artifacts report positive size_bytes'
        : `existing_proof_pack_artifact_non_positive_sizes=${releaseProofPackExistingArtifactRowsNonPositiveSize.join(',')}`,
    },
    {
      id: 'release_proof_pack_existing_artifact_rows_sources_are_absolute',
      ok: !finalStage || releaseProofPackExistingArtifactRowsNonAbsoluteSource.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;existing proof-pack artifact source absolute-path contract enforced on final stage only'
        : releaseProofPackExistingArtifactRowsNonAbsoluteSource.length === 0
        ? 'all existing proof-pack artifact source paths are absolute'
        : `existing_proof_pack_artifact_non_absolute_sources=${releaseProofPackExistingArtifactRowsNonAbsoluteSource.join(',')}`,
    },
    {
      id: 'release_proof_pack_existing_artifact_rows_destinations_are_absolute',
      ok: !finalStage || releaseProofPackExistingArtifactRowsNonAbsoluteDestination.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;existing proof-pack artifact destination absolute-path contract enforced on final stage only'
        : releaseProofPackExistingArtifactRowsNonAbsoluteDestination.length === 0
        ? 'all existing proof-pack artifact destination paths are absolute'
        : `existing_proof_pack_artifact_non_absolute_destinations=${releaseProofPackExistingArtifactRowsNonAbsoluteDestination.join(',')}`,
    },
    {
      id: 'release_proof_pack_summary_artifact_count_present',
      ok: !finalStage || Number.isFinite(Number(releaseProofPackSummary.artifact_count)),
      detail: !finalStage
        ? 'stage=prebundle;proof-pack summary artifact-count presence enforced on final stage only'
        : `summary_artifact_count=${metricText(releaseProofPackSummary.artifact_count)}`,
    },
    {
      id: 'release_proof_pack_summary_artifact_count_matches_rows',
      ok:
        !finalStage ||
        (Number.isFinite(Number(releaseProofPackSummary.artifact_count)) &&
          Number(releaseProofPackSummary.artifact_count) === releaseProofPackArtifactPathRows.length),
      detail: !finalStage
        ? 'stage=prebundle;proof-pack summary artifact-count alignment enforced on final stage only'
        : `summary_artifact_count=${metricText(
            releaseProofPackSummary.artifact_count,
          )};artifact_rows=${releaseProofPackArtifactPathRows.length}`,
    },
    {
      id: 'release_proof_pack_summary_required_missing_present',
      ok: !finalStage || Number.isFinite(Number(releaseProofPackSummary.required_missing)),
      detail: !finalStage
        ? 'stage=prebundle;proof-pack summary required-missing presence enforced on final stage only'
        : `summary_required_missing=${metricText(releaseProofPackSummary.required_missing)}`,
    },
    {
      id: 'release_proof_pack_summary_category_threshold_failure_count_present',
      ok:
        !finalStage ||
        Number.isFinite(Number(releaseProofPackSummary.category_threshold_failure_count)),
      detail: !finalStage
        ? 'stage=prebundle;proof-pack summary category-threshold-failure-count presence enforced on final stage only'
        : `summary_category_threshold_failure_count=${metricText(
            releaseProofPackSummary.category_threshold_failure_count,
          )}`,
    },
    {
      id: 'release_proof_pack_summary_pass_is_boolean',
      ok: !finalStage || typeof releaseProofPackSummary.pass === 'boolean',
      detail: !finalStage
        ? 'stage=prebundle;proof-pack summary pass-type contract enforced on final stage only'
        : `summary_pass_type=${typeof releaseProofPackSummary.pass}`,
    },
    {
      id: 'release_proof_pack_summary_pass_aligned_with_ok',
      ok:
        !finalStage ||
        (typeof releaseProofPackSummary.pass === 'boolean' &&
          typeof releaseProofPack?.ok === 'boolean' &&
          releaseProofPackSummary.pass === releaseProofPack.ok),
      detail: !finalStage
        ? 'stage=prebundle;proof-pack summary pass/ok alignment enforced on final stage only'
        : `summary_pass=${String(releaseProofPackSummary.pass)};proof_pack_ok=${String(
            releaseProofPack?.ok,
          )}`,
    },
    {
      id: 'release_proof_pack_summary_counts_are_nonnegative_integers',
      ok: !finalStage || releaseProofPackSummaryCountsAreNonNegativeIntegers,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack summary integer-count contracts enforced on final stage only'
        : `artifact_count=${metricText(releaseProofPackSummary.artifact_count)};` +
          `required_missing=${metricText(releaseProofPackSummary.required_missing)};` +
          `category_threshold_failure_count=${metricText(releaseProofPackSummary.category_threshold_failure_count)}`,
    },
    {
      id: 'release_proof_pack_required_artifact_rows_have_checksum',
      ok: !finalStage || releaseProofPackRequiredArtifactRowsMissingChecksum.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack required artifact checksum presence enforced on final stage only'
        : releaseProofPackRequiredArtifactRowsMissingChecksum.length === 0
        ? 'all required proof-pack artifact rows declare checksums'
        : `missing_required_artifact_checksums=${releaseProofPackRequiredArtifactRowsMissingChecksum.join(',')}`,
    },
    {
      id: 'release_proof_pack_required_artifact_path_contracts_are_unique',
      ok: releaseProofPackRequiredArtifactPathsAreUnique,
      detail:
        `required_artifact_path_contracts=${RELEASE_PROOF_PACK_REQUIRED_ARTIFACT_PATHS.length};` +
        `unique_required_artifact_path_contracts=${new Set(RELEASE_PROOF_PACK_REQUIRED_ARTIFACT_PATHS).size}`,
    },
    {
      id: 'release_proof_pack_required_artifact_path_contracts_use_core_local_artifacts_prefix',
      ok: releaseProofPackRequiredArtifactPathsAreCanonical,
      detail:
        `required_artifact_path_contracts=${RELEASE_PROOF_PACK_REQUIRED_ARTIFACT_PATHS.join(',')}`,
    },
    {
      id: 'release_proof_pack_required_category_contracts_are_unique',
      ok: releaseProofPackRequiredCategoriesAreUnique,
      detail:
        `required_category_contracts=${RELEASE_PROOF_PACK_REQUIRED_CATEGORIES.length};` +
        `unique_required_category_contracts=${new Set(RELEASE_PROOF_PACK_REQUIRED_CATEGORIES).size}`,
    },
    {
      id: 'release_gate_telemetry_profile_contracts_are_unique',
      ok: releaseGateTelemetryProfilesAreUnique,
      detail:
        `telemetry_profile_contracts=${RELEASE_GATE_TELEMETRY_PROFILES.length};` +
        `unique_telemetry_profile_contracts=${new Set(RELEASE_GATE_TELEMETRY_PROFILES).size}`,
    },
    {
      id: 'release_gate_telemetry_key_contracts_are_unique',
      ok: releaseGateTelemetryKeysAreUnique,
      detail:
        `telemetry_key_contracts=${RELEASE_GATE_TELEMETRY_KEYS.length};` +
        `unique_telemetry_key_contracts=${new Set(RELEASE_GATE_TELEMETRY_KEYS).size}`,
    },
    {
      id: 'release_proof_pack_required_artifact_count_covers_required_path_contracts',
      ok:
        !finalStage ||
        releaseProofPackRequiredArtifactCount >= RELEASE_PROOF_PACK_REQUIRED_ARTIFACT_PATHS.length,
      detail: !finalStage
        ? 'stage=prebundle;required-artifact coverage ratio enforced on final stage only'
        : `required_artifact_count=${releaseProofPackRequiredArtifactCount};` +
          `required_path_contract_count=${RELEASE_PROOF_PACK_REQUIRED_ARTIFACT_PATHS.length}`,
    },
    {
      id: 'release_proof_pack_artifact_rows_cover_required_artifact_rows',
      ok:
        !finalStage ||
        releaseProofPackArtifactPathRows.length >= releaseProofPackRequiredArtifactCount,
      detail: !finalStage
        ? 'stage=prebundle;artifact-row coverage ratio enforced on final stage only'
        : `artifact_rows=${releaseProofPackArtifactPathRows.length};` +
          `required_artifact_rows=${releaseProofPackRequiredArtifactCount}`,
    },
    {
      id: 'release_proof_pack_required_artifact_rows_checksum_is_sha256_hex',
      ok: !finalStage || releaseProofPackRequiredArtifactRowsInvalidChecksum.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack required artifact checksum format enforced on final stage only'
        : releaseProofPackRequiredArtifactRowsInvalidChecksum.length === 0
        ? 'all required proof-pack artifact checksums are sha256 hex digests'
        : `invalid_required_artifact_checksums=${releaseProofPackRequiredArtifactRowsInvalidChecksum.join(',')}`,
    },
    {
      id: 'release_proof_pack_required_artifact_rows_have_positive_size',
      ok: !finalStage || releaseProofPackRequiredArtifactRowsWithNonPositiveSize.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack required artifact size contract enforced on final stage only'
        : releaseProofPackRequiredArtifactRowsWithNonPositiveSize.length === 0
        ? 'all required proof-pack artifacts report positive size_bytes'
        : `non_positive_required_artifact_sizes=${releaseProofPackRequiredArtifactRowsWithNonPositiveSize.join(',')}`,
    },
    {
      id: 'release_proof_pack_required_artifact_rows_sources_within_repo',
      ok: !finalStage || releaseProofPackRequiredArtifactRowsOutsideRepo.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack required artifact source-path repo-boundary contract enforced on final stage only'
        : releaseProofPackRequiredArtifactRowsOutsideRepo.length === 0
        ? 'all required proof-pack artifact source paths remain within repository root'
        : `required_artifact_sources_outside_repo=${releaseProofPackRequiredArtifactRowsOutsideRepo.join(',')}`,
    },
    {
      id: 'release_proof_pack_required_artifact_rows_destinations_within_pack_root',
      ok: !finalStage || releaseProofPackRequiredArtifactRowsOutsidePackRoot.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack required artifact destination-path pack-root contract enforced on final stage only'
        : releaseProofPackRequiredArtifactRowsOutsidePackRoot.length === 0
        ? 'all required proof-pack artifact destination paths remain within pack root'
        : `required_artifact_destinations_outside_pack_root=${releaseProofPackRequiredArtifactRowsOutsidePackRoot.join(',')}`,
    },
    {
      id: 'release_proof_pack_summary_required_missing_zero',
      ok: !finalStage || safeNumber(releaseProofPackSummary.required_missing, Number.NaN) === 0,
      detail: !finalStage
        ? 'stage=prebundle;required_missing_enforced_on_final_stage_only'
        : `required_missing=${metricText(releaseProofPackSummary.required_missing)}`,
    },
    {
      id: 'release_proof_pack_summary_category_threshold_failures_zero',
      ok:
        !finalStage ||
        safeNumber(releaseProofPackSummary.category_threshold_failure_count, Number.NaN) === 0,
      detail: !finalStage
        ? 'stage=prebundle;category_threshold_failures_enforced_on_final_stage_only'
        : `category_threshold_failure_count=${metricText(releaseProofPackSummary.category_threshold_failure_count)}`,
    },
    {
      id: 'release_proof_pack_summary_pass',
      ok: !finalStage || (releaseProofPack?.ok === true && releaseProofPackSummary.pass === true),
      detail: !finalStage
        ? 'stage=prebundle;proof_pack_pass_enforced_on_final_stage_only'
        : `proof_pack_ok=${releaseProofPack?.ok === true};summary_pass=${releaseProofPackSummary.pass === true}`,
    },
    {
      id: 'release_proof_pack_revision_matches_scorecard',
      ok:
        !finalStage ||
        (!!releaseProofPackRevision &&
          !!scorecardRevision &&
          releaseProofPackRevision === scorecardRevision),
      detail: !finalStage
        ? 'stage=prebundle;proof_pack_revision_alignment_enforced_on_final_stage_only'
        : `proof_pack_revision=${releaseProofPackRevision || 'missing'};scorecard_revision=${scorecardRevision || 'missing'}`,
    },
    {
      id: 'release_proof_pack_revision_uses_git_sha_token',
      ok: !finalStage || isGitRevisionToken(releaseProofPackRevision),
      detail: !finalStage
        ? 'stage=prebundle;proof-pack revision token format enforced on final stage only'
        : `proof_pack_revision=${releaseProofPackRevision || 'missing'}`,
    },
    {
      id: 'release_scorecard_revision_uses_git_sha_token',
      ok: !finalStage || isGitRevisionToken(scorecardRevision),
      detail: !finalStage
        ? 'stage=prebundle;scorecard revision token format enforced on final stage only'
        : `scorecard_revision=${scorecardRevision || 'missing'}`,
    },
    {
      id: 'release_verdict_revision_uses_git_sha_token',
      ok: !finalStage || isGitRevisionToken(supportBundleReleaseVerdictRevision),
      detail: !finalStage
        ? 'stage=prebundle;release-verdict revision token format enforced on final stage only'
        : `release_verdict_revision=${supportBundleReleaseVerdictRevision || 'missing'}`,
    },
    {
      id: 'support_bundle_release_proof_pack_revision_uses_git_sha_token',
      ok: !finalStage || isGitRevisionToken(supportBundleReleaseProofPackRevision),
      detail: !finalStage
        ? 'stage=prebundle;support-bundle proof-pack revision token format enforced on final stage only'
        : `support_bundle_release_proof_pack_revision=${supportBundleReleaseProofPackRevision || 'missing'}`,
    },
    {
      id: 'bundled_scorecard_revision_uses_git_sha_token',
      ok: !finalStage || isGitRevisionToken(bundledScorecardRevision),
      detail: !finalStage
        ? 'stage=prebundle;bundled-scorecard revision token format enforced on final stage only'
        : `bundled_scorecard_revision=${bundledScorecardRevision || 'missing'}`,
    },
    {
      id: 'release_proof_pack_category_minimums_present',
      ok:
        !finalStage ||
        RELEASE_PROOF_PACK_REQUIRED_CATEGORIES.every(
          (category) =>
            Number.isFinite(Number(releaseProofPackCategoryCompletenessMin[category])) &&
            Number(releaseProofPackCategoryCompletenessMin[category]) >= 1,
        ),
      detail: !finalStage
        ? 'stage=prebundle;category minimum contracts enforced on final stage only'
        : `category_minimums_present=${RELEASE_PROOF_PACK_REQUIRED_CATEGORIES.filter(
            (category) =>
              Number.isFinite(Number(releaseProofPackCategoryCompletenessMin[category])) &&
              Number(releaseProofPackCategoryCompletenessMin[category]) >= 1,
          ).length}/${RELEASE_PROOF_PACK_REQUIRED_CATEGORIES.length}`,
    },
    {
      id: 'release_proof_pack_category_minimum_keys_cover_required_categories',
      ok:
        !finalStage ||
        RELEASE_PROOF_PACK_REQUIRED_CATEGORIES.every((category) =>
          releaseProofPackCategoryMinimumKeys.includes(category),
        ),
      detail: !finalStage
        ? 'stage=prebundle;proof-pack category-minimum key coverage enforced on final stage only'
        : `category_minimum_keys_present=${RELEASE_PROOF_PACK_REQUIRED_CATEGORIES.filter((category) => releaseProofPackCategoryMinimumKeys.includes(category)).length}/${RELEASE_PROOF_PACK_REQUIRED_CATEGORIES.length}`,
    },
    {
      id: 'release_proof_pack_category_minimum_keys_do_not_include_unknown_categories',
      ok: !finalStage || releaseProofPackCategoryMinimumUnknownKeys.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack category-minimum unknown-key rejection enforced on final stage only'
        : releaseProofPackCategoryMinimumUnknownKeys.length === 0
        ? 'proof-pack category-minimum map has no unknown category keys'
        : `unknown_category_minimum_keys=${releaseProofPackCategoryMinimumUnknownKeys.join(',')}`,
    },
    {
      id: 'release_proof_pack_category_minimum_values_are_numeric_and_nonnegative',
      ok: !finalStage || releaseProofPackCategoryMinimumNonFiniteOrNegative.length === 0,
      detail: !finalStage
        ? 'stage=prebundle;proof-pack category-minimum numeric contracts enforced on final stage only'
        : releaseProofPackCategoryMinimumNonFiniteOrNegative.length === 0
        ? 'proof-pack category-minimum map values are numeric and non-negative'
        : `invalid_category_minimum_values=${releaseProofPackCategoryMinimumNonFiniteOrNegative.join(',')}`,
    },
    {
      id: 'release_proof_pack_required_artifact_count_positive',
      ok: !finalStage || releaseProofPackRequiredArtifactCount > 0,
      detail: !finalStage
        ? 'stage=prebundle;required-artifact-count contract enforced on final stage only'
        : `required_artifact_count=${releaseProofPackRequiredArtifactCount}`,
    },
    {
      id: 'release_proof_pack_categories_cover_required_classes',
      ok:
        !finalStage ||
        RELEASE_PROOF_PACK_REQUIRED_CATEGORIES.every((category) =>
          releaseProofPackCategories.has(category),
        ),
      detail: !finalStage
        ? 'stage=prebundle;proof-pack category coverage enforced on final stage only'
        : `covered_categories=${RELEASE_PROOF_PACK_REQUIRED_CATEGORIES.filter((category) => releaseProofPackCategories.has(category)).join(',') || 'none'}`,
    },
    {
      id: 'release_proof_pack_required_artifact_rows_present',
      ok:
        !finalStage ||
        RELEASE_PROOF_PACK_REQUIRED_ARTIFACT_PATHS.every((artifactPath) => {
          const row = releaseProofPackArtifactRows.get(String(artifactPath));
          return !!row && row.exists === true && row.required === true;
        }),
      detail: !finalStage
        ? 'stage=prebundle;proof_pack_required_artifacts_enforced_on_final_stage_only'
        : `required_artifacts_present=${RELEASE_PROOF_PACK_REQUIRED_ARTIFACT_PATHS.filter((artifactPath) => {
            const row = releaseProofPackArtifactRows.get(String(artifactPath));
            return !!row && row.exists === true && row.required === true;
          }).length}/${RELEASE_PROOF_PACK_REQUIRED_ARTIFACT_PATHS.length}`,
    },
    {
      id: 'release_proof_pack_generated_after_scorecard_stage',
      ok:
        !finalStage ||
        (Number.isFinite(releaseProofPackGeneratedAtMs) &&
          Number.isFinite(scorecardGeneratedAtMs) &&
          scorecardGeneratedAtMs <= releaseProofPackGeneratedAtMs),
      detail: !finalStage
        ? 'stage=prebundle;proof_pack_generation_order_enforced_on_final_stage_only'
        : `scorecard_generated_at=${Number.isFinite(scorecardGeneratedAtMs) ? scorecardGeneratedAtMs : 'missing'};` +
          `proof_pack_generated_at=${Number.isFinite(releaseProofPackGeneratedAtMs) ? releaseProofPackGeneratedAtMs : 'missing'}`,
    },
    {
      id: 'release_evidence_flow_support_bundle_embeds_release_proof_pack_summary',
      ok:
        !finalStage ||
        (supportBundleReleaseProofPack != null &&
          typeof supportBundleReleaseProofPack === 'object'),
      detail: !finalStage
        ? 'stage=prebundle;support-bundle proof-pack summary embedding enforced on final stage only'
        : `support_bundle_release_proof_pack_embedded=${supportBundleReleaseProofPack != null}`,
    },
    {
      id: 'release_evidence_flow_support_bundle_release_proof_pack_required_missing_zero',
      ok:
        !finalStage ||
        safeNumber(supportBundleReleaseProofPack?.summary?.required_missing, Number.NaN) === 0,
      detail: !finalStage
        ? 'stage=prebundle;support-bundle proof-pack required-missing closure enforced on final stage only'
        : `support_bundle_release_proof_pack_required_missing=${metricText(
            supportBundleReleaseProofPack?.summary?.required_missing,
          )}`,
    },
    {
      id: 'release_evidence_flow_support_bundle_release_proof_pack_required_missing_matches_release_proof_pack',
      ok:
        !finalStage ||
        (Number.isFinite(Number(supportBundleReleaseProofPackSummary?.required_missing)) &&
          Number.isFinite(Number(releaseProofPackSummary?.required_missing)) &&
          Number(supportBundleReleaseProofPackSummary?.required_missing) ===
            Number(releaseProofPackSummary?.required_missing)),
      detail: !finalStage
        ? 'stage=prebundle;support-bundle/release-proof-pack required-missing alignment enforced on final stage only'
        : `support_bundle_release_proof_pack_required_missing=${metricText(
            supportBundleReleaseProofPackSummary?.required_missing,
          )};release_proof_pack_required_missing=${metricText(
            releaseProofPackSummary?.required_missing,
          )}`,
    },
    {
      id: 'release_evidence_flow_support_bundle_release_proof_pack_summary_artifact_count_matches_release_proof_pack',
      ok:
        !finalStage ||
        (Number.isFinite(Number(supportBundleReleaseProofPackSummary?.artifact_count)) &&
          Number.isFinite(Number(releaseProofPackSummary?.artifact_count)) &&
          Number(supportBundleReleaseProofPackSummary?.artifact_count) ===
            Number(releaseProofPackSummary?.artifact_count)),
      detail: !finalStage
        ? 'stage=prebundle;support-bundle/release-proof-pack artifact-count alignment enforced on final stage only'
        : `support_bundle_release_proof_pack_artifact_count=${metricText(
            supportBundleReleaseProofPackSummary?.artifact_count,
          )};release_proof_pack_artifact_count=${metricText(
            releaseProofPackSummary?.artifact_count,
          )}`,
    },
    {
      id: 'release_evidence_flow_support_bundle_release_proof_pack_summary_scalars_are_canonical',
      ok:
        !finalStage ||
        supportBundleReleaseProofPackSummaryScalarsAreCanonical,
      detail: !finalStage
        ? 'stage=prebundle;support-bundle release-proof-pack scalar canonicality enforced on final stage only'
        : `required_missing=${metricText(
            supportBundleReleaseProofPackSummary?.required_missing,
          )};category_threshold_failure_count=${metricText(
            supportBundleReleaseProofPackSummary?.category_threshold_failure_count,
          )};pass_type=${typeof supportBundleReleaseProofPackSummary?.pass}`,
    },
    {
      id: 'release_evidence_flow_support_bundle_release_proof_pack_summary_pass',
      ok:
        !finalStage ||
        supportBundleReleaseProofPackSummary?.pass === true,
      detail: !finalStage
        ? 'stage=prebundle;support-bundle proof-pack summary pass enforced on final stage only'
        : `support_bundle_release_proof_pack_summary_pass=${supportBundleReleaseProofPackSummary?.pass === true}`,
    },
    {
      id: 'release_evidence_flow_support_bundle_release_proof_pack_category_threshold_failures_zero',
      ok:
        !finalStage ||
        safeNumber(
          supportBundleReleaseProofPackSummary?.category_threshold_failure_count,
          Number.NaN,
        ) === 0,
      detail: !finalStage
        ? 'stage=prebundle;support-bundle proof-pack category-threshold closure enforced on final stage only'
        : `support_bundle_release_proof_pack_category_threshold_failure_count=${metricText(
            supportBundleReleaseProofPackSummary?.category_threshold_failure_count,
          )}`,
    },
    {
      id: 'release_evidence_flow_support_bundle_release_proof_pack_category_threshold_failure_count_matches_release_proof_pack',
      ok:
        !finalStage ||
        (Number.isFinite(Number(supportBundleReleaseProofPackSummary?.category_threshold_failure_count)) &&
          Number.isFinite(Number(releaseProofPackSummary?.category_threshold_failure_count)) &&
          Number(supportBundleReleaseProofPackSummary?.category_threshold_failure_count) ===
            Number(releaseProofPackSummary?.category_threshold_failure_count)),
      detail: !finalStage
        ? 'stage=prebundle;support-bundle/release-proof-pack category-threshold alignment enforced on final stage only'
        : `support_bundle_release_proof_pack_category_threshold_failure_count=${metricText(
            supportBundleReleaseProofPackSummary?.category_threshold_failure_count,
          )};release_proof_pack_category_threshold_failure_count=${metricText(
            releaseProofPackSummary?.category_threshold_failure_count,
          )}`,
    },
    {
      id: 'release_evidence_flow_support_bundle_release_proof_pack_summary_pass_matches_release_proof_pack',
      ok:
        !finalStage ||
        (typeof supportBundleReleaseProofPackSummary?.pass === 'boolean' &&
          typeof releaseProofPackSummary?.pass === 'boolean' &&
          supportBundleReleaseProofPackSummary?.pass === releaseProofPackSummary?.pass),
      detail: !finalStage
        ? 'stage=prebundle;support-bundle/release-proof-pack pass alignment enforced on final stage only'
        : `support_bundle_release_proof_pack_pass=${supportBundleReleaseProofPackSummary?.pass};release_proof_pack_pass=${releaseProofPackSummary?.pass}`,
    },
    {
      id: 'release_evidence_flow_support_bundle_release_proof_pack_revision_matches_release_proof_pack',
      ok:
        !finalStage ||
        (!!supportBundleReleaseProofPackRevision &&
          !!releaseProofPackRevision &&
          supportBundleReleaseProofPackRevision === releaseProofPackRevision),
      detail: !finalStage
        ? 'stage=prebundle;support-bundle/release-proof-pack revision alignment enforced on final stage only'
        : `support_bundle_release_proof_pack_revision=${supportBundleReleaseProofPackRevision || 'missing'};` +
          `release_proof_pack_revision=${releaseProofPackRevision || 'missing'}`,
    },
    {
      id: 'release_evidence_flow_support_bundle_release_proof_pack_revision_matches_scorecard',
      ok:
        !finalStage ||
        (!!supportBundleReleaseProofPackRevision &&
          !!scorecardRevision &&
          supportBundleReleaseProofPackRevision === scorecardRevision),
      detail: !finalStage
        ? 'stage=prebundle;support-bundle proof-pack/scorecard revision alignment enforced on final stage only'
        : `support_bundle_release_proof_pack_revision=${supportBundleReleaseProofPackRevision || 'missing'};` +
          `scorecard_revision=${scorecardRevision || 'missing'}`,
    },
    {
      id: 'release_evidence_flow_support_bundle_release_proof_pack_generated_at_present',
      ok:
        !finalStage ||
        Number.isFinite(supportBundleReleaseProofPackGeneratedAtMs),
      detail: !finalStage
        ? 'stage=prebundle;support-bundle proof-pack generated-at contract enforced on final stage only'
        : `support_bundle_release_proof_pack_generated_at=${Number.isFinite(supportBundleReleaseProofPackGeneratedAtMs) ? supportBundleReleaseProofPackGeneratedAtMs : 'missing'}`,
    },
    {
      id: 'release_evidence_flow_release_proof_pack_generated_at_present',
      ok:
        !finalStage ||
        Number.isFinite(releaseProofPackGeneratedAtMs),
      detail: !finalStage
        ? 'stage=prebundle;release-proof-pack generated-at contract enforced on final stage only'
        : `release_proof_pack_generated_at=${Number.isFinite(releaseProofPackGeneratedAtMs) ? releaseProofPackGeneratedAtMs : 'missing'}`,
    },
    {
      id: 'release_evidence_flow_scorecard_generated_at_present',
      ok:
        !finalStage ||
        Number.isFinite(scorecardGeneratedAtMs),
      detail: !finalStage
        ? 'stage=prebundle;scorecard generated-at contract enforced on final stage only'
        : `scorecard_generated_at=${Number.isFinite(scorecardGeneratedAtMs) ? scorecardGeneratedAtMs : 'missing'}`,
    },
    {
      id: 'release_evidence_flow_support_bundle_generated_at_present',
      ok:
        !finalStage ||
        Number.isFinite(supportBundleGeneratedAtMs),
      detail: !finalStage
        ? 'stage=prebundle;support-bundle generated-at contract enforced on final stage only'
        : `support_bundle_generated_at=${Number.isFinite(supportBundleGeneratedAtMs) ? supportBundleGeneratedAtMs : 'missing'}`,
    },
    {
      id: 'release_evidence_flow_support_bundle_release_proof_pack_generated_after_bundled_scorecard',
      ok:
        !finalStage ||
        !Number.isFinite(bundledScorecardGeneratedAtMs) ||
        (Number.isFinite(supportBundleReleaseProofPackGeneratedAtMs) &&
          bundledScorecardGeneratedAtMs <= supportBundleReleaseProofPackGeneratedAtMs),
      detail: !finalStage
        ? 'stage=prebundle;support-bundle embedded scorecard/proof-pack ordering enforced on final stage only'
        : `bundled_scorecard_generated_at=${Number.isFinite(bundledScorecardGeneratedAtMs) ? bundledScorecardGeneratedAtMs : 'missing'};` +
          `support_bundle_release_proof_pack_generated_at=${Number.isFinite(supportBundleReleaseProofPackGeneratedAtMs) ? supportBundleReleaseProofPackGeneratedAtMs : 'missing'}`,
    },
    {
      id: 'release_evidence_flow_bundled_scorecard_revision_matches_scorecard',
      ok:
        !finalStage ||
        (!!bundledScorecardRevision &&
          !!scorecardRevision &&
          bundledScorecardRevision === scorecardRevision),
      detail: !finalStage
        ? 'stage=prebundle;bundled-scorecard/scorecard revision alignment enforced on final stage only'
        : `bundled_scorecard_revision=${bundledScorecardRevision || 'missing'};` +
          `scorecard_revision=${scorecardRevision || 'missing'}`,
    },
    {
      id: 'release_evidence_flow_support_bundle_generated_after_proof_pack',
      ok:
        !finalStage ||
        (Number.isFinite(supportBundleGeneratedAtMs) &&
          Number.isFinite(releaseProofPackGeneratedAtMs) &&
          releaseProofPackGeneratedAtMs <= supportBundleGeneratedAtMs),
      detail: !finalStage
        ? 'stage=prebundle;support-bundle/proof-pack ordering enforced on final stage only'
        : `proof_pack_generated_at=${Number.isFinite(releaseProofPackGeneratedAtMs) ? releaseProofPackGeneratedAtMs : 'missing'};` +
          `support_bundle_generated_at=${Number.isFinite(supportBundleGeneratedAtMs) ? supportBundleGeneratedAtMs : 'missing'}`,
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
  checks.push(...checkReleaseGateTelemetryThresholds());
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
