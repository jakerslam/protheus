#!/usr/bin/env node
'use strict';

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult } from '../../lib/result.ts';

const DEFAULT_OUT = path.join(process.cwd(), 'core/local/artifacts/release_verdict_current.json');
const DEFAULT_POLICY = path.join(process.cwd(), 'client/runtime/config/production_readiness_closure_policy.json');
const DEFAULT_GATE_REGISTRY = path.join(process.cwd(), 'tests/tooling/config/tooling_gate_registry.json');
const DEFAULT_VERIFY_PROFILES = path.join(process.cwd(), 'tests/tooling/config/verify_profiles.json');
const GATE_ID_TOKEN_REGEX = /^([a-z0-9_]+|ops:[a-z0-9:-]+|chaos:[a-z0-9:-]+|state:[a-z0-9:-]+)$/;
const RELEASE_VERDICT_ALLOWED_PATH_PREFIXES = [
  'core/local/artifacts/',
  'client/local/state/',
  'client/runtime/local/state/',
];
const RELEASE_VERDICT_REQUIRED_GATE_ORDER = [
  'release_policy_gate',
  'chaos:continuous:gate',
  'state:kernel:replay',
  'ops:production-topology:gate',
  'ops:stateful-upgrade-rollback:gate',
  'ops:assimilation:v1:support:guard',
  'ops:orchestration:hidden-state:guard',
  'ops:release-blockers:gate',
  'ops:release-hardening-window:guard',
  'ops:runtime-proof:verify',
  'ops:layer2:parity:guard',
  'ops:layer2:receipt:replay',
  'ops:trusted-core:report',
  'ops:release:proof-pack',
  'ops:release:scorecard:gate',
  'ops:production-closure:gate',
  'ops:release:rc-rehearsal',
] as const;
const RELEASE_VERDICT_REQUIRED_RELEASE_PROFILE_GATES = [
  'release_policy_gate',
  'ops:runtime-proof:verify',
  'ops:layer2:parity:guard',
  'ops:layer2:receipt:replay',
  'ops:trusted-core:report',
  'ops:release:proof-pack',
  'ops:release:scorecard:gate',
  'ops:production-closure:gate',
  'ops:release:rc-rehearsal',
  'ops:release:verdict',
] as const;
const RELEASE_VERDICT_POLICY_REQUIRED_RC_STEP_EXTRAS = [
  'dr:gameday:gate',
  'audit:client-layer-boundary',
  'ops:ipc-bridge:soak',
] as const;
const RELEASE_VERDICT_POLICY_REQUIRED_FILES = [
  'client/runtime/config/production_readiness_closure_policy.json',
  'tests/tooling/config/tooling_gate_registry.json',
  'tests/tooling/config/verify_profiles.json',
] as const;
const RELEASE_VERDICT_POLICY_REQUIRED_PACKAGE_SCRIPTS = [
  'ops:release:verdict',
  'ops:runtime-proof:verify',
] as const;
const RELEASE_VERDICT_POLICY_REQUIRED_CI_INVOCATIONS = [
  'ops:release:verdict',
  'ops:runtime-proof:verify',
] as const;
const RELEASE_VERDICT_REQUIRED_GATE_ARTIFACT_PATHS: Record<string, string> = {
  'release_policy_gate': 'core/local/artifacts/release_policy_gate_current.json',
  'chaos:continuous:gate': 'client/local/state/runtime_systems/RUNTIME-SYSTEMS-OPS-CONTINUOUS_CHAOS_RESILIENCE/latest.json',
  'state:kernel:replay': 'client/local/state/runtime_systems/RUNTIME-SYSTEMS-OPS-STATE_KERNEL/latest.json',
  'ops:production-topology:gate': 'core/local/artifacts/production_topology_diagnostic_current.json',
  'ops:stateful-upgrade-rollback:gate': 'core/local/artifacts/stateful_upgrade_rollback_gate_current.json',
  'ops:assimilation:v1:support:guard': 'core/local/artifacts/assimilation_v1_support_guard_current.json',
  'ops:orchestration:hidden-state:guard': 'core/local/artifacts/orchestration_hidden_state_guard_current.json',
  'ops:release-blockers:gate': 'core/local/artifacts/release_blocker_rubric_current.json',
  'ops:release-hardening-window:guard': 'core/local/artifacts/release_hardening_window_guard_current.json',
  'ops:runtime-proof:verify': 'core/local/artifacts/runtime_proof_verify_current.json',
  'ops:layer2:parity:guard': 'core/local/artifacts/layer2_lane_parity_guard_current.json',
  'ops:layer2:receipt:replay': 'core/local/artifacts/layer2_receipt_replay_current.json',
  'ops:trusted-core:report': 'core/local/artifacts/runtime_trusted_core_report_current.json',
  'ops:release:proof-pack': 'core/local/artifacts/release_proof_pack_current.json',
  'ops:release:scorecard:gate': 'client/runtime/local/state/release/scorecard/release_scorecard.json',
  'ops:production-closure:gate': 'core/local/artifacts/production_readiness_closure_gate_current.json',
  'ops:release:rc-rehearsal': 'core/local/artifacts/release_candidate_dress_rehearsal_current.json',
};

function parseArgs(argv: string[]) {
  const parsed = parseStrictOutArgs(argv, { out: DEFAULT_OUT, strict: false });
  return {
    strict: parsed.strict,
    out: parsed.out || DEFAULT_OUT,
    policyPath: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY, 400),
    gateRegistryPath: cleanText(readFlag(argv, 'gate-registry') || DEFAULT_GATE_REGISTRY, 400),
    verifyProfilesPath: cleanText(readFlag(argv, 'verify-profiles') || DEFAULT_VERIFY_PROFILES, 400),
    rootPath: cleanText(readFlag(argv, 'root') || '', 400),
  };
}

function resolveMaybe(root: string, maybePath: string): string {
  if (!maybePath) return '';
  if (path.isAbsolute(maybePath)) return maybePath;
  return path.resolve(root, maybePath);
}

function readJsonMaybe(filePath: string): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function isCanonicalRelativePathToken(
  value: unknown,
  requiredPrefix = '',
  requiredSuffix = '',
): boolean {
  const token = cleanText(value || '', 500);
  if (!token) return false;
  if (path.isAbsolute(token)) return false;
  if (token.includes('\\')) return false;
  if (token.includes('..')) return false;
  if (token.includes('//')) return false;
  if (/\s/.test(token)) return false;
  if (requiredPrefix && !token.startsWith(requiredPrefix)) return false;
  if (requiredSuffix && !token.endsWith(requiredSuffix)) return false;
  return true;
}

function isReleaseVerdictArtifactPathAllowed(relPath: string): boolean {
  return RELEASE_VERDICT_ALLOWED_PATH_PREFIXES.some((prefix) => relPath.startsWith(prefix));
}

function isReleaseVerdictArtifactFilenameAllowed(relPath: string): boolean {
  const base = path.posix.basename(relPath);
  return base.endsWith('_current.json') || base === 'latest.json' || base === 'release_scorecard.json';
}

function artifactOk(gateId: string, payload: any): boolean {
  switch (gateId) {
    case 'release_policy_gate':
      return payload?.ok === true;
    case 'ops:production-topology:gate':
      return (
        payload?.ok === true &&
        payload?.supported_production_topology === true &&
        Array.isArray(payload?.degraded_flags) &&
        payload.degraded_flags.length === 0
      );
    case 'chaos:continuous:gate':
    case 'state:kernel:replay':
      return payload?.ok === true;
    case 'ops:stateful-upgrade-rollback:gate':
    case 'ops:assimilation:v1:support:guard':
    case 'ops:release-blockers:gate':
    case 'ops:release-hardening-window:guard':
    case 'ops:layer2:parity:guard':
    case 'ops:layer2:receipt:replay':
    case 'ops:trusted-core:report':
    case 'ops:release:scorecard:gate':
      return payload?.ok === true;
    case 'ops:runtime-proof:verify': {
      const profiles = Array.isArray(payload?.profile_runs) ? payload.profile_runs : [];
      const proofTrack = String(payload?.summary?.proof_track || '').trim();
      const profileCount = Number(payload?.summary?.profile_count || 0);
      const empiricalOk = profiles.every((row: any) => row?.empirical_sample_points_ok === true);
      return payload?.ok === true && proofTrack === 'dual' && profileCount >= 3 && empiricalOk;
    }
    case 'ops:release:proof-pack': {
      const requiredMissing = Number(
        payload?.summary?.required_missing ?? (Array.isArray(payload?.required_missing) ? payload.required_missing.length : 0),
      );
      const categoryThresholdFailures = Number(
        payload?.summary?.category_threshold_failure_count ??
          (Array.isArray(payload?.category_threshold_failures) ? payload.category_threshold_failures.length : 0),
      );
      return payload?.ok === true && requiredMissing === 0 && categoryThresholdFailures === 0;
    }
    case 'ops:orchestration:hidden-state:guard':
      return payload?.summary?.pass === true || payload?.summary?.violation_count === 0;
    case 'ops:production-closure:gate':
      return payload?.summary?.pass === true || payload?.ok === true;
    case 'ops:release:rc-rehearsal':
      return payload?.ok === true && payload?.summary?.candidate_ready === true;
    default:
      return payload?.ok === true;
  }
}

function artifactStrict(payload: any): boolean {
  return payload?.strict === true || payload?.inputs?.strict === true;
}

function fileDigest(filePath: string): string {
  return crypto.createHash('sha256').update(fs.readFileSync(filePath)).digest('hex');
}

export function buildReport(rawArgs = parseArgs(process.argv.slice(2))) {
  const args = typeof rawArgs === 'object' && rawArgs ? rawArgs : parseArgs(process.argv.slice(2));
  const root = path.resolve(args.rootPath || process.cwd());
  const policyPath = resolveMaybe(root, args.policyPath || DEFAULT_POLICY);
  const gateRegistryPath = resolveMaybe(root, args.gateRegistryPath || DEFAULT_GATE_REGISTRY);
  const verifyProfilesPath = resolveMaybe(root, args.verifyProfilesPath || DEFAULT_VERIFY_PROFILES);
  const policy = readJsonMaybe(policyPath) || {};
  const gateRegistryParsed = readJsonMaybe(gateRegistryPath);
  const verifyProfilesParsed = readJsonMaybe(verifyProfilesPath);
  const gateRegistryGates =
    gateRegistryParsed?.gates && typeof gateRegistryParsed.gates === 'object' && !Array.isArray(gateRegistryParsed.gates)
      ? gateRegistryParsed.gates
      : {};
  const gateRegistryGateIds = Object.keys(gateRegistryGates || {}).map((gateId) => cleanText(gateId || '', 180)).filter(Boolean);
  const gateRegistryGateIdsInvalid = gateRegistryGateIds.filter((gateId) => !GATE_ID_TOKEN_REGEX.test(gateId));
  const releaseProfileGateIds = Array.isArray(verifyProfilesParsed?.profiles?.release?.gate_ids)
    ? verifyProfilesParsed.profiles.release.gate_ids.map((gateId: any) => cleanText(gateId || '', 180)).filter(Boolean)
    : [];
  const runtimeProofProfileGateIds = Array.isArray(verifyProfilesParsed?.profiles?.runtime-proof?.gate_ids)
    ? verifyProfilesParsed.profiles['runtime-proof'].gate_ids
        .map((gateId: any) => cleanText(gateId || '', 180))
        .filter(Boolean)
    : [];
  const verdictPolicy = policy.release_verdict || {};
  const requiredGateArtifacts = verdictPolicy.required_gate_artifacts || {};
  const checksumArtifactPaths = Array.isArray(verdictPolicy.checksum_artifact_paths)
    ? verdictPolicy.checksum_artifact_paths
    : [];
  const outPath = resolveMaybe(root, args.out || DEFAULT_OUT);
  const outRel = path.relative(root, outPath).replace(/\\/g, '/');
  const outInArtifacts =
    outRel.length > 0 &&
    !outRel.startsWith('../') &&
    !path.isAbsolute(outRel) &&
    outRel.startsWith('core/local/artifacts/');
  const policyRel = path.relative(root, policyPath).replace(/\\/g, '/');
  const policyInRepo =
    policyRel.length > 0 &&
    !policyRel.startsWith('../') &&
    !path.isAbsolute(policyRel);
  const gateRegistryRel = path.relative(root, gateRegistryPath).replace(/\\/g, '/');
  const gateRegistryInRepo =
    gateRegistryRel.length > 0 &&
    !gateRegistryRel.startsWith('../') &&
    !path.isAbsolute(gateRegistryRel);
  const verifyProfilesRel = path.relative(root, verifyProfilesPath).replace(/\\/g, '/');
  const verifyProfilesInRepo =
    verifyProfilesRel.length > 0 &&
    !verifyProfilesRel.startsWith('../') &&
    !path.isAbsolute(verifyProfilesRel);
  const outPathMatchesPolicyArtifact =
    isCanonicalRelativePathToken(verdictPolicy?.artifact_path, 'core/local/artifacts/', '.json') &&
    outRel === cleanText(verdictPolicy?.artifact_path || '', 500);
  const policyParsed = readJsonMaybe(policyPath);
  const verdictPolicyKeyset = (() => {
    if (!verdictPolicy || typeof verdictPolicy !== 'object' || Array.isArray(verdictPolicy)) {
      return { missing: ['script', 'artifact_path', 'required_gate_artifacts', 'checksum_artifact_paths'], unexpected: [] };
    }
    const keys = Object.keys(verdictPolicy);
    const expected = new Set(['script', 'artifact_path', 'required_gate_artifacts', 'checksum_artifact_paths']);
    const missing = Array.from(expected).filter((key) => !keys.includes(key));
    const unexpected = keys.filter((key) => !expected.has(key));
    return { missing, unexpected };
  })();
  const requiredGateArtifactEntries = Object.entries(requiredGateArtifacts).map(([gateId, relPath]) => ({
    gateId: cleanText(gateId || '', 180),
    relPath: cleanText(relPath || '', 500),
  }));
  const requiredGateIds = requiredGateArtifactEntries.map((row) => row.gateId);
  const requiredGateIdsDuplicate = requiredGateIds.filter(
    (row, idx, arr) => row && arr.indexOf(row) !== idx,
  );
  const requiredGateArtifactPaths = requiredGateArtifactEntries.map((row) => row.relPath);
  const requiredGateArtifactPathsDuplicate = requiredGateArtifactPaths.filter(
    (row, idx, arr) => row && arr.indexOf(row) !== idx,
  );
  const requiredGateArtifactBasenames = requiredGateArtifactPaths.map((relPath) => path.posix.basename(relPath));
  const requiredGateArtifactBasenamesDuplicate = requiredGateArtifactBasenames.filter(
    (row, idx, arr) => row && arr.indexOf(row) !== idx,
  );
  const requiredGateArtifactMissingBaseline = [
    'release_policy_gate',
    'ops:runtime-proof:verify',
    'ops:release:proof-pack',
    'ops:release:scorecard:gate',
    'ops:production-closure:gate',
    'ops:release:rc-rehearsal',
  ].filter((gateId) => !requiredGateIds.includes(gateId));
  const requiredGateIdsMissingInRegistry = requiredGateIds.filter(
    (gateId) => !gateRegistryGateIds.includes(gateId),
  );
  const requiredGateRegistryArtifactBindingMismatches = requiredGateArtifactEntries
    .map((row) => {
      const registryRow = gateRegistryGates[row.gateId];
      if (!registryRow || typeof registryRow !== 'object' || Array.isArray(registryRow)) {
        return `${row.gateId}:missing_registry_entry`;
      }
      const registryArtifacts = Array.isArray(registryRow.artifact_paths)
        ? registryRow.artifact_paths.map((relPath: any) => cleanText(relPath || '', 500)).filter(Boolean)
        : [];
      if (!registryArtifacts.includes(row.relPath)) {
        return `${row.gateId}:${row.relPath || 'missing_required_path'}->${registryArtifacts.join('|') || 'missing_registry_artifact_paths'}`;
      }
      return '';
    })
    .filter(Boolean);
  const releaseProfileGateIdsDuplicate = releaseProfileGateIds.filter(
    (gateId, idx, arr) => gateId && arr.indexOf(gateId) !== idx,
  );
  const releaseProfileGateIdsInvalid = releaseProfileGateIds.filter(
    (gateId) => !GATE_ID_TOKEN_REGEX.test(gateId),
  );
  const releaseProfileGateIdsMissingInRegistry = releaseProfileGateIds.filter(
    (gateId) => !gateRegistryGateIds.includes(gateId),
  );
  const releaseProfileRequiredIndexes = RELEASE_VERDICT_REQUIRED_RELEASE_PROFILE_GATES.map((gateId) =>
    releaseProfileGateIds.indexOf(gateId),
  );
  const releaseProfileRequiredOrderValid =
    releaseProfileRequiredIndexes.every((idx) => idx >= 0) &&
    releaseProfileRequiredIndexes.every((idx, index, arr) => index === 0 || idx > arr[index - 1]);
  const requiredGateIdsMissingInReleaseProfile = requiredGateIds.filter(
    (gateId) => !releaseProfileGateIds.includes(gateId),
  );
  const releaseProfileRequiredMissing = RELEASE_VERDICT_REQUIRED_RELEASE_PROFILE_GATES.filter(
    (gateId) => !releaseProfileGateIds.includes(gateId),
  );
  const releaseProfileRequiredMissingInRegistry = RELEASE_VERDICT_REQUIRED_RELEASE_PROFILE_GATES.filter(
    (gateId) => !gateRegistryGateIds.includes(gateId),
  );
  const runtimeProofRequiredSubset = [
    'ops:runtime-proof:verify',
    'ops:layer2:parity:guard',
    'ops:layer2:receipt:replay',
    'ops:trusted-core:report',
  ];
  const runtimeProofProfileGateIdsDuplicate = runtimeProofProfileGateIds.filter(
    (gateId, idx, arr) => gateId && arr.indexOf(gateId) !== idx,
  );
  const runtimeProofProfileGateIdsInvalid = runtimeProofProfileGateIds.filter(
    (gateId) => !GATE_ID_TOKEN_REGEX.test(gateId),
  );
  const runtimeProofProfileGateIdsMissingInRegistry = runtimeProofProfileGateIds.filter(
    (gateId) => !gateRegistryGateIds.includes(gateId),
  );
  const runtimeProofRequiredIndexes = runtimeProofRequiredSubset.map((gateId) =>
    runtimeProofProfileGateIds.indexOf(gateId),
  );
  const runtimeProofRequiredOrderValid =
    runtimeProofRequiredIndexes.every((idx) => idx >= 0) &&
    runtimeProofRequiredIndexes.every((idx, index, arr) => index === 0 || idx > arr[index - 1]);
  const runtimeProofRequiredMissing = runtimeProofRequiredSubset.filter(
    (gateId) => !runtimeProofProfileGateIds.includes(gateId),
  );
  const runtimeProofMissingInReleaseProfile = runtimeProofProfileGateIds.filter(
    (gateId) => !releaseProfileGateIds.includes(gateId),
  );
  const runtimeProofIndexesInReleaseProfile = runtimeProofProfileGateIds.map((gateId) =>
    releaseProfileGateIds.indexOf(gateId),
  );
  const runtimeProofRelativeOrderInReleaseProfileValid =
    runtimeProofIndexesInReleaseProfile.every((idx) => idx >= 0) &&
    runtimeProofIndexesInReleaseProfile.every((idx, index, arr) => index === 0 || idx > arr[index - 1]);
  const policyRequiredReleaseVerifyGateIds = Array.isArray(policy?.required_verify_profile_gate_ids?.release)
    ? policy.required_verify_profile_gate_ids.release.map((gateId: any) => cleanText(gateId || '', 180)).filter(Boolean)
    : [];
  const policyRequiredReleaseVerifyGateIdsDuplicate = policyRequiredReleaseVerifyGateIds.filter(
    (gateId, idx, arr) => gateId && arr.indexOf(gateId) !== idx,
  );
  const policyRequiredReleaseVerifyGateIdsInvalid = policyRequiredReleaseVerifyGateIds.filter(
    (gateId) => !GATE_ID_TOKEN_REGEX.test(gateId),
  );
  const policyRequiredReleaseVerifyGateIdsMissingInRegistry = policyRequiredReleaseVerifyGateIds.filter(
    (gateId) => !gateRegistryGateIds.includes(gateId),
  );
  const policyRequiredReleaseVerifyGateIdsMissingInReleaseProfile = policyRequiredReleaseVerifyGateIds.filter(
    (gateId) => !releaseProfileGateIds.includes(gateId),
  );
  const policyRequiredReleaseVerifyIndexesInReleaseProfile = policyRequiredReleaseVerifyGateIds.map((gateId) =>
    releaseProfileGateIds.indexOf(gateId),
  );
  const policyRequiredReleaseVerifyOrderInReleaseProfileValid =
    policyRequiredReleaseVerifyIndexesInReleaseProfile.every((idx) => idx >= 0) &&
    policyRequiredReleaseVerifyIndexesInReleaseProfile.every((idx, index, arr) => index === 0 || idx > arr[index - 1]);
  const rcPolicyRequiredStepGateIds = Array.isArray(policy?.release_candidate_rehearsal?.required_step_gate_ids)
    ? policy.release_candidate_rehearsal.required_step_gate_ids
        .map((gateId: any) => cleanText(gateId || '', 180))
        .filter(Boolean)
    : [];
  const rcPolicyRequiredStepGateIdsDuplicate = rcPolicyRequiredStepGateIds.filter(
    (gateId, idx, arr) => gateId && arr.indexOf(gateId) !== idx,
  );
  const rcPolicyRequiredStepGateIdsInvalid = rcPolicyRequiredStepGateIds.filter(
    (gateId) => !GATE_ID_TOKEN_REGEX.test(gateId),
  );
  const rcPolicyRequiredStepGateIdsMissingInRegistry = rcPolicyRequiredStepGateIds.filter(
    (gateId) => !gateRegistryGateIds.includes(gateId),
  );
  const rcPolicyRequiredStepGateIdsMissingInReleaseProfile = rcPolicyRequiredStepGateIds.filter(
    (gateId) => !releaseProfileGateIds.includes(gateId),
  );
  const rcPolicyMissingRequiredGateArtifacts = requiredGateIds
    .filter((gateId) => gateId !== 'ops:release:rc-rehearsal')
    .filter((gateId) => !rcPolicyRequiredStepGateIds.includes(gateId));
  const rcPolicyExtraStepGateIds = rcPolicyRequiredStepGateIds.filter(
    (gateId) => !requiredGateIds.includes(gateId),
  );
  const rcPolicyExtraStepExpected = Array.from(RELEASE_VERDICT_POLICY_REQUIRED_RC_STEP_EXTRAS);
  const rcPolicyExtraStepSetMatches =
    rcPolicyExtraStepGateIds.length === rcPolicyExtraStepExpected.length &&
    rcPolicyExtraStepGateIds.every((gateId, idx) => gateId === rcPolicyExtraStepExpected[idx]);
  const requiredGateIdsMissingExpected = RELEASE_VERDICT_REQUIRED_GATE_ORDER.filter(
    (gateId) => !requiredGateIds.includes(gateId),
  );
  const requiredGateIdsUnexpected = requiredGateIds.filter(
    (gateId) => !RELEASE_VERDICT_REQUIRED_GATE_ORDER.includes(gateId as any),
  );
  const requiredGateOrderMatchesExpected =
    requiredGateIds.length === RELEASE_VERDICT_REQUIRED_GATE_ORDER.length &&
    requiredGateIds.every((gateId, index) => gateId === RELEASE_VERDICT_REQUIRED_GATE_ORDER[index]);
  const requiredGatePathMismatches = requiredGateArtifactEntries
    .map((row) => {
      const expectedPath = RELEASE_VERDICT_REQUIRED_GATE_ARTIFACT_PATHS[row.gateId] || '';
      if (!expectedPath || row.relPath !== expectedPath) {
        return `${row.gateId}:${row.relPath || 'missing'}->${expectedPath || 'missing_expected_path'}`;
      }
      return '';
    })
    .filter(Boolean);
  const requiredGatePathsOutsideAllowedPrefixes = requiredGateArtifactPaths.filter(
    (relPath) => !isReleaseVerdictArtifactPathAllowed(relPath),
  );
  const requiredGatePathsInvalidFilename = requiredGateArtifactPaths.filter(
    (relPath) => !isReleaseVerdictArtifactFilenameAllowed(relPath),
  );
  const requiredGateFamilyCounts = {
    ops: requiredGateIds.filter((gateId) => gateId.startsWith('ops:')).length,
    chaos: requiredGateIds.filter((gateId) => gateId.startsWith('chaos:')).length,
    state: requiredGateIds.filter((gateId) => gateId.startsWith('state:')).length,
    release_family: requiredGateIds.filter(
      (gateId) => gateId === 'release_policy_gate' || gateId.startsWith('ops:release:'),
    ).length,
  };
  const requiredGateLayer2PairPresent =
    requiredGateIds.includes('ops:layer2:parity:guard') &&
    requiredGateIds.includes('ops:layer2:receipt:replay');
  const checksumPathsTrimmed = checksumArtifactPaths.map((row: any) => cleanText(row || '', 500));
  const checksumPathsDuplicate = checksumPathsTrimmed.filter(
    (row, idx, arr) => row && arr.indexOf(row) !== idx,
  );
  const checksumPathsNotInRequired = checksumPathsTrimmed.filter(
    (row) => row && !requiredGateArtifactPaths.includes(row),
  );
  const checksumPathsMissingRequired = requiredGateArtifactPaths.filter(
    (relPath) => relPath && !checksumPathsTrimmed.includes(relPath),
  );
  const checksumOrderMatchesRequired =
    checksumPathsTrimmed.length === requiredGateArtifactPaths.length &&
    checksumPathsTrimmed.every((relPath, index) => relPath === requiredGateArtifactPaths[index]);
  const checksumPathsOutsideAllowedPrefixes = checksumPathsTrimmed.filter(
    (relPath) => !isReleaseVerdictArtifactPathAllowed(relPath),
  );
  const checksumPathsInvalidFilename = checksumPathsTrimmed.filter(
    (relPath) => !isReleaseVerdictArtifactFilenameAllowed(relPath),
  );
  const checksumBasenames = checksumPathsTrimmed.map((relPath) => path.posix.basename(relPath));
  const checksumBasenamesDuplicate = checksumBasenames.filter(
    (row, idx, arr) => row && arr.indexOf(row) !== idx,
  );
  const checksumRequiredBaselineMissing = [
    'core/local/artifacts/release_policy_gate_current.json',
    'core/local/artifacts/runtime_proof_verify_current.json',
    'core/local/artifacts/release_proof_pack_current.json',
    'client/runtime/local/state/release/scorecard/release_scorecard.json',
    'core/local/artifacts/production_readiness_closure_gate_current.json',
    'core/local/artifacts/release_candidate_dress_rehearsal_current.json',
  ].filter((relPath) => !checksumPathsTrimmed.includes(relPath));
  const rcPath = resolveMaybe(root, requiredGateArtifacts['ops:release:rc-rehearsal'] || '');
  const rcPayload = readJsonMaybe(rcPath) || {};
  const rcSteps = Array.isArray(rcPayload?.steps) ? rcPayload.steps : [];
  const rcStepMap = new Map(rcSteps.map((row: any) => [String(row?.gate_id || ''), row]));
  const rcStepGateIds = rcSteps.map((row: any) => cleanText(row?.gate_id || '', 180)).filter(Boolean);
  const rcStepGateIdsDuplicate = rcStepGateIds.filter(
    (row, idx, arr) => arr.indexOf(row) !== idx,
  );
  const rcRequiredStepGateIds = requiredGateIds.filter((gateId) => gateId !== 'ops:release:rc-rehearsal');
  const rcStepGateIdsMissing = rcRequiredStepGateIds.filter((gateId) => !rcStepGateIds.includes(gateId));
  const rcStepGateIdsMissingInRegistry = rcStepGateIds.filter(
    (gateId) => !gateRegistryGateIds.includes(gateId),
  );
  const rcStepCountExpected = rcRequiredStepGateIds.length;
  const rcStepRowsAreObjects = rcSteps.every(
    (row: any) => row && typeof row === 'object' && !Array.isArray(row),
  );
  const rcStepGateIdsInvalidToken = rcStepGateIds.filter((gateId) => !GATE_ID_TOKEN_REGEX.test(gateId));
  const rcStepMissingOkBooleanCount = rcSteps.filter(
    (row: any) => typeof row?.ok !== 'boolean',
  ).length;
  const rcStepGateIdsUnexpected = rcStepGateIds.filter((gateId) => !requiredGateIds.includes(gateId));
  const rcStepGateIdsMissingInPolicy = rcStepGateIds.filter(
    (gateId) => !rcPolicyRequiredStepGateIds.includes(gateId),
  );
  const rcPolicyRequiredStepGateIdsMissingInRcSteps = rcPolicyRequiredStepGateIds.filter(
    (gateId) => !rcStepGateIds.includes(gateId),
  );
  const rcPolicyRequiredStepIndexesInRc = rcPolicyRequiredStepGateIds.map((gateId) =>
    rcStepGateIds.indexOf(gateId),
  );
  const rcPolicyRequiredStepOrderInRcValid =
    rcPolicyRequiredStepIndexesInRc.every((idx) => idx >= 0) &&
    rcPolicyRequiredStepIndexesInRc.every((idx, index, arr) => index === 0 || idx > arr[index - 1]);
  const rcStepOrders = rcSteps.map((row: any) => Number(row?.order));
  const rcStepOrderInvalid = rcStepOrders.filter(
    (order) => !Number.isInteger(order) || order <= 0,
  );
  const rcStepOrderDuplicates = rcStepOrders.filter(
    (order, idx, arr) => Number.isInteger(order) && order > 0 && arr.indexOf(order) !== idx,
  );
  const rcStepOrdersSorted = rcStepOrders
    .filter((order) => Number.isInteger(order) && order > 0)
    .sort((a, b) => a - b);
  const rcStepOrderContiguous =
    rcStepOrdersSorted.length === rcSteps.length &&
    rcStepOrdersSorted.every((order, index) => order === index + 1);
  const rcFailures = Array.isArray(rcPayload?.failures) ? rcPayload.failures : [];
  const rcFailureRowsAreObjects = rcFailures.every(
    (row: any) => row && typeof row === 'object' && !Array.isArray(row),
  );
  const rcFailureGateIds = rcFailures.map((row: any) => cleanText(row?.gate_id || '', 180)).filter(Boolean);
  const rcFailureGateIdsInvalid = rcFailureGateIds.filter((gateId) => !GATE_ID_TOKEN_REGEX.test(gateId));
  const rcFailureGateIdsDuplicate = rcFailureGateIds.filter(
    (gateId, idx, arr) => arr.indexOf(gateId) !== idx,
  );
  const rcFailureGateIdsMissingInSteps = rcFailureGateIds.filter(
    (gateId) => !rcStepGateIds.includes(gateId),
  );
  const rcFailureOrders = rcFailures.map((row: any) => Number(row?.order));
  const rcFailureOrderMissingInSteps = rcFailureOrders.filter(
    (order) => !Number.isInteger(order) || order <= 0 || !rcStepOrders.includes(order),
  );
  const rcSummaryStepCount = Number(rcPayload?.summary?.step_count ?? -1);
  const rcSummaryFailedCount = Number(rcPayload?.summary?.failed_count ?? -1);
  const rcSummaryRequiredStepCount = Number(rcPayload?.summary?.required_step_count ?? -1);
  const rcSummaryRequiredStepsSatisfied = rcPayload?.summary?.required_steps_satisfied === true;
  const rcSummaryCandidateReady = rcPayload?.summary?.candidate_ready === true;
  const rcDerivedRequiredStepsSatisfied = rcPolicyRequiredStepGateIdsMissingInRcSteps.length === 0;
  const policyRequiredFiles = Array.isArray(policy?.required_files)
    ? policy.required_files.map((row: any) => cleanText(row || '', 500)).filter(Boolean)
    : [];
  const policyRequiredFilesDuplicate = policyRequiredFiles.filter(
    (filePath, idx, arr) => filePath && arr.indexOf(filePath) !== idx,
  );
  const policyRequiredFilesInvalid = policyRequiredFiles.filter(
    (filePath) => !isCanonicalRelativePathToken(filePath),
  );
  const policyRequiredFilesMissingBaseline = RELEASE_VERDICT_POLICY_REQUIRED_FILES.filter(
    (filePath) => !policyRequiredFiles.includes(filePath),
  );
  const policyRequiredPackageScripts = Array.isArray(policy?.required_package_scripts)
    ? policy.required_package_scripts.map((row: any) => cleanText(row || '', 220)).filter(Boolean)
    : [];
  const policyRequiredPackageScriptsDuplicate = policyRequiredPackageScripts.filter(
    (scriptId, idx, arr) => scriptId && arr.indexOf(scriptId) !== idx,
  );
  const policyRequiredPackageScriptsInvalid = policyRequiredPackageScripts.filter(
    (scriptId) => !scriptId || /\s/.test(scriptId) || scriptId !== scriptId.trim(),
  );
  const policyRequiredPackageScriptsMissingBaseline = RELEASE_VERDICT_POLICY_REQUIRED_PACKAGE_SCRIPTS.filter(
    (scriptId) => !policyRequiredPackageScripts.includes(scriptId),
  );
  const policyRequiredCiInvocations = Array.isArray(policy?.required_ci_invocations)
    ? policy.required_ci_invocations.map((row: any) => cleanText(row || '', 220)).filter(Boolean)
    : [];
  const policyRequiredCiInvocationsDuplicate = policyRequiredCiInvocations.filter(
    (scriptId, idx, arr) => scriptId && arr.indexOf(scriptId) !== idx,
  );
  const policyRequiredCiInvocationsInvalid = policyRequiredCiInvocations.filter(
    (scriptId) => !scriptId || /\s/.test(scriptId) || scriptId !== scriptId.trim(),
  );
  const policyRequiredCiInvocationsMissingBaseline = RELEASE_VERDICT_POLICY_REQUIRED_CI_INVOCATIONS.filter(
    (scriptId) => !policyRequiredCiInvocations.includes(scriptId),
  );
  const requiredGateRegistryRows = requiredGateIds.map((gateId) => ({
    gateId,
    row: gateRegistryGates[gateId],
  }));
  const requiredGateRegistryMissingRows = requiredGateRegistryRows
    .filter((entry) => !entry.row || typeof entry.row !== 'object' || Array.isArray(entry.row))
    .map((entry) => entry.gateId);
  const requiredGateRegistryOwnerInvalid = requiredGateRegistryRows
    .filter((entry) => entry.row && cleanText(entry.row.owner || '', 120) !== 'ops')
    .map((entry) => entry.gateId);
  const requiredGateRegistryDescriptionInvalid = requiredGateRegistryRows
    .filter((entry) => {
      if (!entry.row) return false;
      const description = cleanText(entry.row.description || '', 500);
      return !description || description !== description.trim() || /^todo|tbd|placeholder$/i.test(description);
    })
    .map((entry) => entry.gateId);
  const requiredGateRegistryTimeoutInvalid = requiredGateRegistryRows
    .filter((entry) => {
      if (!entry.row) return false;
      const timeoutSec = Number(entry.row.timeout_sec);
      return !Number.isInteger(timeoutSec) || timeoutSec <= 0 || timeoutSec > 900;
    })
    .map((entry) => entry.gateId);
  const requiredGateRegistryTimeoutEnvInvalid = requiredGateRegistryRows
    .filter((entry) => {
      if (!entry.row) return false;
      const timeoutEnv = cleanText(entry.row.timeout_env || '', 120);
      return !timeoutEnv || !/^[A-Z0-9_]+$/.test(timeoutEnv);
    })
    .map((entry) => entry.gateId);
  const requiredGateRegistrySelectorInvalid = requiredGateRegistryRows
    .filter((entry) => {
      if (!entry.row) return false;
      const hasCommand = Array.isArray(entry.row.command);
      const hasScript = cleanText(entry.row.script || '', 220).length > 0;
      return Number(hasCommand) + Number(hasScript) !== 1;
    })
    .map((entry) => entry.gateId);
  const requiredGateRegistryCommandShapeInvalid = requiredGateRegistryRows
    .filter((entry) => {
      if (!entry.row) return false;
      const hasCommand = Array.isArray(entry.row.command);
      if (!hasCommand) return false;
      const command = entry.row.command as any[];
      return (
        command.length === 0 ||
        command.some((token) => {
          const normalized = cleanText(token || '', 220);
          return !normalized || /\s/.test(normalized) && normalized !== '&&';
        })
      );
    })
    .map((entry) => entry.gateId);
  const requiredGateRegistryScriptShapeInvalid = requiredGateRegistryRows
    .filter((entry) => {
      if (!entry.row) return false;
      const script = cleanText(entry.row.script || '', 220);
      if (!script) return false;
      return /\s/.test(script) || script !== script.trim();
    })
    .map((entry) => entry.gateId);
  const requiredGateRegistryArtifactArrayInvalid = requiredGateRegistryRows
    .filter((entry) => !Array.isArray(entry.row?.artifact_paths) || entry.row.artifact_paths.length === 0)
    .map((entry) => entry.gateId);
  const requiredGateRegistryArtifactTokenInvalid = requiredGateRegistryRows
    .filter((entry) => {
      if (!Array.isArray(entry.row?.artifact_paths)) return false;
      return entry.row.artifact_paths
        .map((artifactPath: any) => cleanText(artifactPath || '', 500))
        .some((artifactPath: string) => !artifactPath || !isCanonicalRelativePathToken(artifactPath));
    })
    .map((entry) => entry.gateId);
  const requiredGateRegistryArtifactDuplicateInvalid = requiredGateRegistryRows
    .filter((entry) => {
      if (!Array.isArray(entry.row?.artifact_paths)) return false;
      const normalized = entry.row.artifact_paths
        .map((artifactPath: any) => cleanText(artifactPath || '', 500))
        .filter(Boolean);
      return normalized.some((artifactPath: string, idx: number, arr: string[]) => arr.indexOf(artifactPath) !== idx);
    })
    .map((entry) => entry.gateId);
  const releaseProfileRegistryRows = releaseProfileGateIds.map((gateId) => ({
    gateId,
    row: gateRegistryGates[gateId],
  }));
  const releaseProfileRegistryMissingRows = releaseProfileRegistryRows
    .filter((entry) => !entry.row || typeof entry.row !== 'object' || Array.isArray(entry.row))
    .map((entry) => entry.gateId);
  const releaseProfileRegistryOwnerInvalid = releaseProfileRegistryRows
    .filter((entry) => entry.row && cleanText(entry.row.owner || '', 120) !== 'ops')
    .map((entry) => entry.gateId);
  const releaseProfileRegistryDescriptionInvalid = releaseProfileRegistryRows
    .filter((entry) => {
      if (!entry.row) return false;
      const description = cleanText(entry.row.description || '', 500);
      return !description || description !== description.trim() || /^todo|tbd|placeholder$/i.test(description);
    })
    .map((entry) => entry.gateId);
  const releaseProfileRegistryTimeoutInvalid = releaseProfileRegistryRows
    .filter((entry) => {
      if (!entry.row) return false;
      const timeoutSec = Number(entry.row.timeout_sec);
      return !Number.isInteger(timeoutSec) || timeoutSec <= 0 || timeoutSec > 1800;
    })
    .map((entry) => entry.gateId);
  const releaseProfileRegistrySelectorInvalid = releaseProfileRegistryRows
    .filter((entry) => {
      if (!entry.row) return false;
      const hasCommand = Array.isArray(entry.row.command);
      const hasScript = cleanText(entry.row.script || '', 220).length > 0;
      return Number(hasCommand) + Number(hasScript) !== 1;
    })
    .map((entry) => entry.gateId);
  const runtimeProofRegistryRows = runtimeProofProfileGateIds.map((gateId) => ({
    gateId,
    row: gateRegistryGates[gateId],
  }));
  const runtimeProofRegistryMissingRows = runtimeProofRegistryRows
    .filter((entry) => !entry.row || typeof entry.row !== 'object' || Array.isArray(entry.row))
    .map((entry) => entry.gateId);
  const runtimeProofRegistryOwnerInvalid = runtimeProofRegistryRows
    .filter((entry) => entry.row && cleanText(entry.row.owner || '', 120) !== 'ops')
    .map((entry) => entry.gateId);
  const runtimeProofRegistryTimeoutInvalid = runtimeProofRegistryRows
    .filter((entry) => {
      if (!entry.row) return false;
      const timeoutSec = Number(entry.row.timeout_sec);
      return !Number.isInteger(timeoutSec) || timeoutSec <= 0 || timeoutSec > 1800;
    })
    .map((entry) => entry.gateId);
  const runtimeProofRegistrySelectorInvalid = runtimeProofRegistryRows
    .filter((entry) => {
      if (!entry.row) return false;
      const hasCommand = Array.isArray(entry.row.command);
      const hasScript = cleanText(entry.row.script || '', 220).length > 0;
      return Number(hasCommand) + Number(hasScript) !== 1;
    })
    .map((entry) => entry.gateId);
  const rcStepDurationInvalid = rcSteps
    .filter((row: any) => {
      const durationMs = Number(row?.duration_ms);
      return !Number.isInteger(durationMs) || durationMs < 0;
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcStepExitCodeInvalid = rcSteps
    .filter((row: any) => {
      const ok = row?.ok === true;
      const exitCode = Number(row?.exit_code);
      if (!Number.isInteger(exitCode)) return true;
      return ok ? exitCode !== 0 : exitCode === 0;
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}:${Number(row?.exit_code)}`);
  const rcStepFailureMessageInvalid = rcSteps
    .filter((row: any) => {
      const ok = row?.ok === true;
      const failure = cleanText(row?.failure || '', 4000);
      return ok ? failure.length > 0 : failure.length === 0;
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcStepArtifactPathsArrayInvalid = rcSteps
    .filter((row: any) => !Array.isArray(row?.artifact_paths))
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcStepArtifactPathsTokenInvalid = rcSteps
    .filter((row: any) => {
      if (!Array.isArray(row?.artifact_paths)) return false;
      return row.artifact_paths
        .map((artifactPath: any) => cleanText(artifactPath || '', 500))
        .some((artifactPath: string) => !artifactPath || !isCanonicalRelativePathToken(artifactPath));
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcStepArtifactPathsDuplicateInvalid = rcSteps
    .filter((row: any) => {
      if (!Array.isArray(row?.artifact_paths)) return false;
      const normalized = row.artifact_paths
        .map((artifactPath: any) => cleanText(artifactPath || '', 500))
        .filter(Boolean);
      return normalized.some((artifactPath: string, idx: number, arr: string[]) => arr.indexOf(artifactPath) !== idx);
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcStepFailedIdsArrayInvalid = rcSteps
    .filter((row: any) => !Array.isArray(row?.failed_ids))
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcStepFailedIdsTokenInvalid = rcSteps
    .filter((row: any) => {
      if (!Array.isArray(row?.failed_ids)) return false;
      return row.failed_ids
        .map((checkId: any) => cleanText(checkId || '', 220))
        .some((checkId: string) => !checkId || !GATE_ID_TOKEN_REGEX.test(checkId));
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcStepFailedIdsDuplicateInvalid = rcSteps
    .filter((row: any) => {
      if (!Array.isArray(row?.failed_ids)) return false;
      const normalized = row.failed_ids
        .map((checkId: any) => cleanText(checkId || '', 220))
        .filter(Boolean);
      return normalized.some((checkId: string, idx: number, arr: string[]) => arr.indexOf(checkId) !== idx);
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcStepDegradedFlagsArrayInvalid = rcSteps
    .filter((row: any) => !Array.isArray(row?.degraded_flags))
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcStepDegradedFlagsTokenInvalid = rcSteps
    .filter((row: any) => {
      if (!Array.isArray(row?.degraded_flags)) return false;
      return row.degraded_flags
        .map((flag: any) => cleanText(flag || '', 180))
        .some((flag: string) => !flag || !/^[a-z0-9_:-]+$/.test(flag));
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcStepDegradedFlagsDuplicateInvalid = rcSteps
    .filter((row: any) => {
      if (!Array.isArray(row?.degraded_flags)) return false;
      const normalized = row.degraded_flags
        .map((flag: any) => cleanText(flag || '', 180))
        .filter(Boolean);
      return normalized.some((flag: string, idx: number, arr: string[]) => arr.indexOf(flag) !== idx);
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcFailedStepOrders = rcSteps
    .filter((row: any) => row?.ok === false)
    .map((row: any) => Number(row?.order))
    .filter((order: number) => Number.isInteger(order) && order > 0);
  const rcFailedStepGateIds = rcSteps
    .filter((row: any) => row?.ok === false)
    .map((row: any) => cleanText(row?.gate_id || '', 180))
    .filter(Boolean);
  const rcFailureOkFalseInvalid = rcFailures
    .filter((row: any) => row?.ok !== false)
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcFailureExitCodeInvalid = rcFailures
    .filter((row: any) => {
      const exitCode = Number(row?.exit_code);
      return !Number.isInteger(exitCode) || exitCode === 0;
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}:${Number(row?.exit_code)}`);
  const rcFailureArtifactPathsArrayInvalid = rcFailures
    .filter((row: any) => !Array.isArray(row?.artifact_paths))
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcFailureArtifactPathsTokenInvalid = rcFailures
    .filter((row: any) => {
      if (!Array.isArray(row?.artifact_paths)) return false;
      return row.artifact_paths
        .map((artifactPath: any) => cleanText(artifactPath || '', 500))
        .some((artifactPath: string) => !artifactPath || !isCanonicalRelativePathToken(artifactPath));
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcFailureArtifactPathsDuplicateInvalid = rcFailures
    .filter((row: any) => {
      if (!Array.isArray(row?.artifact_paths)) return false;
      const normalized = row.artifact_paths
        .map((artifactPath: any) => cleanText(artifactPath || '', 500))
        .filter(Boolean);
      return normalized.some((artifactPath: string, idx: number, arr: string[]) => arr.indexOf(artifactPath) !== idx);
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcFailureOrdersMissingInFailedSteps = rcFailureOrders.filter(
    (order) => !Number.isInteger(order) || order <= 0 || !rcFailedStepOrders.includes(order),
  );
  const rcFailureGateIdsMissingInFailedSteps = rcFailureGateIds.filter(
    (gateId) => !rcFailedStepGateIds.includes(gateId),
  );
  const rcRecovery = rcPayload?.recovery_rehearsal;
  const rcChaos = rcPayload?.chaos;
  const rcReplay = rcPayload?.replay;
  const rcTopology = rcPayload?.topology;
  const rcClientBoundary = rcPayload?.client_boundary;
  const rcHiddenState = rcPayload?.hidden_state;
  const rcLayer2Parity = rcPayload?.layer2_parity;
  const rcLayer2ReceiptReplay = rcPayload?.layer2_receipt_replay;
  const rcTrustedCore = rcPayload?.trusted_core;
  const rcProofPack = rcPayload?.proof_pack;
  const rcRecoveryIsObject = Boolean(rcRecovery && typeof rcRecovery === 'object' && !Array.isArray(rcRecovery));
  const rcChaosIsObject = Boolean(rcChaos && typeof rcChaos === 'object' && !Array.isArray(rcChaos));
  const rcReplayIsObject = Boolean(rcReplay && typeof rcReplay === 'object' && !Array.isArray(rcReplay));
  const rcTopologyIsObject = Boolean(rcTopology && typeof rcTopology === 'object' && !Array.isArray(rcTopology));
  const rcClientBoundaryIsObject = Boolean(
    rcClientBoundary && typeof rcClientBoundary === 'object' && !Array.isArray(rcClientBoundary),
  );
  const rcHiddenStateIsObject = Boolean(
    rcHiddenState && typeof rcHiddenState === 'object' && !Array.isArray(rcHiddenState),
  );
  const rcLayer2ParityIsObject = Boolean(
    rcLayer2Parity && typeof rcLayer2Parity === 'object' && !Array.isArray(rcLayer2Parity),
  );
  const rcLayer2ReceiptReplayIsObject = Boolean(
    rcLayer2ReceiptReplay && typeof rcLayer2ReceiptReplay === 'object' && !Array.isArray(rcLayer2ReceiptReplay),
  );
  const rcTrustedCoreIsObject = Boolean(
    rcTrustedCore && typeof rcTrustedCore === 'object' && !Array.isArray(rcTrustedCore),
  );
  const rcProofPackIsObject = Boolean(
    rcProofPack && typeof rcProofPack === 'object' && !Array.isArray(rcProofPack),
  );
  const rcRecoveryGateState = cleanText(rcRecovery?.gate_state || '', 180);
  const rcRecoverySemanticValid =
    typeof rcRecovery?.ok === 'boolean' &&
    rcRecoveryGateState.length > 0 &&
    /^[a-z0-9_:-]+$/.test(rcRecoveryGateState);
  const rcChaosPayloadType = cleanText(rcChaos?.payload_type || '', 220);
  const rcChaosSemanticValid = typeof rcChaos?.ok === 'boolean' && rcChaosPayloadType.length > 0;
  const rcReplayPayloadType = cleanText(rcReplay?.payload_type || '', 220);
  const rcReplaySemanticValid = typeof rcReplay?.ok === 'boolean' && rcReplayPayloadType.length > 0;
  const rcTopologyDegradedFlags = Array.isArray(rcTopology?.degraded_flags)
    ? rcTopology.degraded_flags.map((flag: any) => cleanText(flag || '', 180)).filter(Boolean)
    : [];
  const rcTopologyDegradedFlagsInvalid = rcTopologyDegradedFlags.filter((flag: string) => !/^[a-z0-9_:-]+$/.test(flag));
  const rcTopologyDegradedFlagsDuplicate = rcTopologyDegradedFlags.filter(
    (flag: string, idx: number, arr: string[]) => arr.indexOf(flag) !== idx,
  );
  const rcClientBoundaryFailedIds = Array.isArray(rcClientBoundary?.failed_ids)
    ? rcClientBoundary.failed_ids.map((id: any) => cleanText(id || '', 220)).filter(Boolean)
    : [];
  const rcClientBoundaryFailedIdsInvalid = rcClientBoundaryFailedIds.filter(
    (id: string) => !id || /\s/.test(id) || id !== id.trim(),
  );
  const rcClientBoundaryFailedIdsDuplicate = rcClientBoundaryFailedIds.filter(
    (id: string, idx: number, arr: string[]) => arr.indexOf(id) !== idx,
  );
  const rcClientBoundaryConsistency =
    typeof rcClientBoundary?.ok === 'boolean' &&
    (rcClientBoundary.ok === false || rcClientBoundaryFailedIds.length === 0);
  const rcHiddenStateFailure = cleanText(rcHiddenState?.failure || '', 4000);
  const rcHiddenStateConsistency =
    typeof rcHiddenState?.ok === 'boolean' &&
    (rcHiddenState.ok === false ? rcHiddenStateFailure.length > 0 : rcHiddenStateFailure.length === 0);
  const rcLayer2ParityArtifactPaths = Array.isArray(rcLayer2Parity?.artifact_paths)
    ? rcLayer2Parity.artifact_paths.map((artifactPath: any) => cleanText(artifactPath || '', 500)).filter(Boolean)
    : [];
  const rcLayer2ParityArtifactPathsInvalid = rcLayer2ParityArtifactPaths.filter(
    (artifactPath: string) => !isCanonicalRelativePathToken(artifactPath),
  );
  const rcLayer2ParityArtifactPathsDuplicate = rcLayer2ParityArtifactPaths.filter(
    (artifactPath: string, idx: number, arr: string[]) => arr.indexOf(artifactPath) !== idx,
  );
  const rcLayer2ReceiptReplayArtifactPaths = Array.isArray(rcLayer2ReceiptReplay?.artifact_paths)
    ? rcLayer2ReceiptReplay.artifact_paths.map((artifactPath: any) => cleanText(artifactPath || '', 500)).filter(Boolean)
    : [];
  const rcLayer2ReceiptReplayArtifactPathsInvalid = rcLayer2ReceiptReplayArtifactPaths.filter(
    (artifactPath: string) => !isCanonicalRelativePathToken(artifactPath),
  );
  const rcLayer2ReceiptReplayArtifactPathsDuplicate = rcLayer2ReceiptReplayArtifactPaths.filter(
    (artifactPath: string, idx: number, arr: string[]) => arr.indexOf(artifactPath) !== idx,
  );
  const rcTrustedCoreArtifactPaths = Array.isArray(rcTrustedCore?.artifact_paths)
    ? rcTrustedCore.artifact_paths.map((artifactPath: any) => cleanText(artifactPath || '', 500)).filter(Boolean)
    : [];
  const rcTrustedCoreArtifactPathsInvalid = rcTrustedCoreArtifactPaths.filter(
    (artifactPath: string) => !isCanonicalRelativePathToken(artifactPath),
  );
  const rcTrustedCoreArtifactPathsDuplicate = rcTrustedCoreArtifactPaths.filter(
    (artifactPath: string, idx: number, arr: string[]) => arr.indexOf(artifactPath) !== idx,
  );
  const rcProofPackArtifactPaths = Array.isArray(rcProofPack?.artifact_paths)
    ? rcProofPack.artifact_paths.map((artifactPath: any) => cleanText(artifactPath || '', 500)).filter(Boolean)
    : [];
  const rcProofPackArtifactPathsInvalid = rcProofPackArtifactPaths.filter(
    (artifactPath: string) => !isCanonicalRelativePathToken(artifactPath),
  );
  const rcProofPackArtifactPathsDuplicate = rcProofPackArtifactPaths.filter(
    (artifactPath: string, idx: number, arr: string[]) => arr.indexOf(artifactPath) !== idx,
  );
  const rcTrustedCoreProofPackSemanticValid =
    typeof rcTrustedCore?.ok === 'boolean' &&
    typeof rcProofPack?.ok === 'boolean' &&
    rcTrustedCoreArtifactPathsInvalid.length === 0 &&
    rcTrustedCoreArtifactPathsDuplicate.length === 0 &&
    rcProofPackArtifactPathsInvalid.length === 0 &&
    rcProofPackArtifactPathsDuplicate.length === 0 &&
    (rcTrustedCore.ok === false || rcTrustedCoreArtifactPaths.length > 0) &&
    (rcProofPack.ok === false || rcProofPackArtifactPaths.length > 0);
  const rcPayloadType = cleanText(rcPayload?.type || '', 160);
  const rcPayloadGeneratedAtRaw = cleanText(rcPayload?.generated_at || '', 120);
  const rcPayloadGeneratedAtMs = Date.parse(rcPayloadGeneratedAtRaw);
  const rcPayloadGeneratedAtValid = Number.isFinite(rcPayloadGeneratedAtMs);
  const rcInputs = rcPayload?.inputs;
  const rcInputsIsObject = Boolean(rcInputs && typeof rcInputs === 'object' && !Array.isArray(rcInputs));
  const rcInputRegistryPath = cleanText(rcInputs?.registry_path || '', 500);
  const rcInputRegistryPathValid =
    isCanonicalRelativePathToken(rcInputRegistryPath, '', '.json') &&
    rcInputRegistryPath === 'tests/tooling/config/tooling_gate_registry.json';
  const rcInputActivateHardeningWindowValid = typeof rcInputs?.activate_hardening_window === 'boolean';
  const rcInputRequiredStepGateIds = Array.isArray(rcInputs?.required_step_gate_ids)
    ? rcInputs.required_step_gate_ids.map((gateId: any) => cleanText(gateId || '', 180)).filter(Boolean)
    : [];
  const rcInputRequiredStepGateIdsInvalid = rcInputRequiredStepGateIds.filter(
    (gateId) => !GATE_ID_TOKEN_REGEX.test(gateId),
  );
  const rcInputRequiredStepGateIdsDuplicate = rcInputRequiredStepGateIds.filter(
    (gateId, idx, arr) => gateId && arr.indexOf(gateId) !== idx,
  );
  const rcInputRequiredStepSetMatchesPolicy =
    rcInputRequiredStepGateIds.length === rcPolicyRequiredStepGateIds.length &&
    rcInputRequiredStepGateIds.every((gateId) => rcPolicyRequiredStepGateIds.includes(gateId));
  const rcInputRequiredStepOrderMatchesPolicy =
    rcInputRequiredStepGateIds.length === rcPolicyRequiredStepGateIds.length &&
    rcInputRequiredStepGateIds.every((gateId, idx) => gateId === rcPolicyRequiredStepGateIds[idx]);
  const rcSummaryCountsSemanticValid =
    Number.isInteger(rcSummaryStepCount) &&
    Number.isInteger(rcSummaryFailedCount) &&
    Number.isInteger(rcSummaryRequiredStepCount) &&
    rcSummaryStepCount >= 0 &&
    rcSummaryFailedCount >= 0 &&
    rcSummaryRequiredStepCount >= 0 &&
    rcSummaryFailedCount <= rcSummaryStepCount &&
    rcSummaryRequiredStepCount <= rcSummaryStepCount;
  const rcSummaryFailedMatchesFailedSteps = rcSummaryFailedCount === rcFailedStepOrders.length;
  const rcFailureDurationInvalid = rcFailures
    .filter((row: any) => {
      const durationMs = Number(row?.duration_ms);
      return !Number.isInteger(durationMs) || durationMs < 0;
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcFailureFailureMessageInvalid = rcFailures
    .filter((row: any) => cleanText(row?.failure || '', 4000).length === 0)
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcFailureFailedIdsArrayInvalid = rcFailures
    .filter((row: any) => !Array.isArray(row?.failed_ids))
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcFailureFailedIdsTokenInvalid = rcFailures
    .filter((row: any) => {
      if (!Array.isArray(row?.failed_ids)) return false;
      return row.failed_ids
        .map((checkId: any) => cleanText(checkId || '', 220))
        .some((checkId: string) => !checkId || /\s/.test(checkId) || checkId !== checkId.trim());
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcFailureFailedIdsDuplicateInvalid = rcFailures
    .filter((row: any) => {
      if (!Array.isArray(row?.failed_ids)) return false;
      const normalized = row.failed_ids
        .map((checkId: any) => cleanText(checkId || '', 220))
        .filter(Boolean);
      return normalized.some((checkId: string, idx: number, arr: string[]) => arr.indexOf(checkId) !== idx);
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcFailureDegradedFlagsArrayInvalid = rcFailures
    .filter((row: any) => !Array.isArray(row?.degraded_flags))
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcFailureDegradedFlagsTokenInvalid = rcFailures
    .filter((row: any) => {
      if (!Array.isArray(row?.degraded_flags)) return false;
      return row.degraded_flags
        .map((flag: any) => cleanText(flag || '', 180))
        .some((flag: string) => !flag || !/^[a-z0-9_:-]+$/.test(flag));
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcFailureDegradedFlagsDuplicateInvalid = rcFailures
    .filter((row: any) => {
      if (!Array.isArray(row?.degraded_flags)) return false;
      const normalized = row.degraded_flags
        .map((flag: any) => cleanText(flag || '', 180))
        .filter(Boolean);
      return normalized.some((flag: string, idx: number, arr: string[]) => arr.indexOf(flag) !== idx);
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcTopArtifactPaths = Array.isArray(rcPayload?.artifact_paths)
    ? rcPayload.artifact_paths.map((artifactPath: any) => cleanText(artifactPath || '', 500)).filter(Boolean)
    : [];
  const rcTopArtifactPathsInvalid = rcTopArtifactPaths.filter(
    (artifactPath: string) => !isCanonicalRelativePathToken(artifactPath),
  );
  const rcTopArtifactPathsDuplicate = rcTopArtifactPaths.filter(
    (artifactPath: string, idx: number, arr: string[]) => arr.indexOf(artifactPath) !== idx,
  );
  const rcTopArtifactPathsMissingRequired = requiredGateArtifactPaths.filter(
    (artifactPath) => !rcTopArtifactPaths.includes(artifactPath),
  );
  const rcFailedStepOrdersMissingInFailures = rcFailedStepOrders.filter(
    (order) => !rcFailureOrders.includes(order),
  );
  const rcFailedStepGateIdsMissingInFailures = rcFailedStepGateIds.filter(
    (gateId) => !rcFailureGateIds.includes(gateId),
  );
  const rcStepOrderArrayMismatch = rcSteps
    .filter((row: any, idx: number) => Number(row?.order) !== idx + 1)
    .map((row: any, idx: number) => `${idx + 1}:${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcStepRowsSortedByOrder = rcStepOrders.every(
    (order, idx, arr) => idx === 0 || order >= arr[idx - 1],
  );
  const rcFailureOrderDuplicates = rcFailureOrders.filter(
    (order, idx, arr) => Number.isInteger(order) && order > 0 && arr.indexOf(order) !== idx,
  );
  const rcFailureRowsSortedByOrder = rcFailureOrders.every(
    (order, idx, arr) => idx === 0 || order >= arr[idx - 1],
  );
  const rcFailureOrderGateMismatch = rcFailures
    .filter((row: any) => {
      const order = Number(row?.order);
      if (!Number.isInteger(order) || order <= 0) return true;
      const step = rcSteps.find((candidate: any) => Number(candidate?.order) === order);
      if (!step) return true;
      return cleanText(step?.gate_id || '', 180) !== cleanText(row?.gate_id || '', 180);
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcFailureOrderFailedStateMismatch = rcFailures
    .filter((row: any) => {
      const order = Number(row?.order);
      if (!Number.isInteger(order) || order <= 0) return true;
      const step = rcSteps.find((candidate: any) => Number(candidate?.order) === order);
      if (!step) return true;
      return step?.ok !== false;
    })
    .map((row: any) => `${Number(row?.order || -1)}:${cleanText(row?.gate_id || '', 180) || 'missing'}`);
  const rcStepArtifactPathsMissingFromTop = rcSteps
    .flatMap((row: any) => {
      if (!Array.isArray(row?.artifact_paths)) return [];
      const order = Number(row?.order || -1);
      const gateId = cleanText(row?.gate_id || '', 180) || 'missing';
      return row.artifact_paths
        .map((artifactPath: any) => cleanText(artifactPath || '', 500))
        .filter(Boolean)
        .filter((artifactPath: string) => !rcTopArtifactPaths.includes(artifactPath))
        .map((artifactPath: string) => `${order}:${gateId}:${artifactPath}`);
    });
  const rcFailureArtifactPathsMissingFromTop = rcFailures
    .flatMap((row: any) => {
      if (!Array.isArray(row?.artifact_paths)) return [];
      const order = Number(row?.order || -1);
      const gateId = cleanText(row?.gate_id || '', 180) || 'missing';
      return row.artifact_paths
        .map((artifactPath: any) => cleanText(artifactPath || '', 500))
        .filter(Boolean)
        .filter((artifactPath: string) => !rcTopArtifactPaths.includes(artifactPath))
        .map((artifactPath: string) => `${order}:${gateId}:${artifactPath}`);
    });
  const rcChaosArtifactPaths = Array.isArray(rcChaos?.artifact_paths)
    ? rcChaos.artifact_paths.map((artifactPath: any) => cleanText(artifactPath || '', 500)).filter(Boolean)
    : [];
  const rcChaosArtifactPathsInvalid = rcChaosArtifactPaths.filter(
    (artifactPath: string) => !isCanonicalRelativePathToken(artifactPath),
  );
  const rcChaosArtifactPathsDuplicate = rcChaosArtifactPaths.filter(
    (artifactPath: string, idx: number, arr: string[]) => arr.indexOf(artifactPath) !== idx,
  );
  const rcChaosArtifactPathsMissingFromTop = rcChaosArtifactPaths.filter(
    (artifactPath: string) => !rcTopArtifactPaths.includes(artifactPath),
  );
  const rcReplayArtifactPaths = Array.isArray(rcReplay?.artifact_paths)
    ? rcReplay.artifact_paths.map((artifactPath: any) => cleanText(artifactPath || '', 500)).filter(Boolean)
    : [];
  const rcReplayArtifactPathsInvalid = rcReplayArtifactPaths.filter(
    (artifactPath: string) => !isCanonicalRelativePathToken(artifactPath),
  );
  const rcReplayArtifactPathsDuplicate = rcReplayArtifactPaths.filter(
    (artifactPath: string, idx: number, arr: string[]) => arr.indexOf(artifactPath) !== idx,
  );
  const rcReplayArtifactPathsMissingFromTop = rcReplayArtifactPaths.filter(
    (artifactPath: string) => !rcTopArtifactPaths.includes(artifactPath),
  );
  const rcLayer2ParityArtifactPathsMissingFromTop = rcLayer2ParityArtifactPaths.filter(
    (artifactPath: string) => !rcTopArtifactPaths.includes(artifactPath),
  );
  const rcLayer2ReceiptReplayArtifactPathsMissingFromTop = rcLayer2ReceiptReplayArtifactPaths.filter(
    (artifactPath: string) => !rcTopArtifactPaths.includes(artifactPath),
  );
  const rcTrustedCoreArtifactPathsMissingFromTop = rcTrustedCoreArtifactPaths.filter(
    (artifactPath: string) => !rcTopArtifactPaths.includes(artifactPath),
  );
  const rcProofPackArtifactPathsMissingFromTop = rcProofPackArtifactPaths.filter(
    (artifactPath: string) => !rcTopArtifactPaths.includes(artifactPath),
  );
  const rcChaosExpectedArtifact = cleanText(requiredGateArtifacts['chaos:continuous:gate'] || '', 500);
  const rcReplayExpectedArtifact = cleanText(requiredGateArtifacts['state:kernel:replay'] || '', 500);
  const rcChaosReplayExpectedArtifactsPresent =
    rcChaosExpectedArtifact.length > 0 &&
    rcReplayExpectedArtifact.length > 0 &&
    rcChaosArtifactPaths.includes(rcChaosExpectedArtifact) &&
    rcReplayArtifactPaths.includes(rcReplayExpectedArtifact);
  const checks: Array<{ id: string; ok: boolean; detail: string }> = [
    {
      id: 'release_verdict_out_path_json_contract',
      ok: outPath.endsWith('.json'),
      detail: outRel || outPath,
    },
    {
      id: 'release_verdict_out_path_artifacts_scope_contract',
      ok: outInArtifacts,
      detail: outRel || outPath,
    },
    {
      id: 'release_verdict_policy_path_json_contract',
      ok: policyPath.endsWith('.json'),
      detail: policyRel || policyPath,
    },
    {
      id: 'release_verdict_policy_path_repo_scope_contract',
      ok: policyInRepo,
      detail: policyRel || policyPath,
    },
    {
      id: 'release_verdict_policy_path_expected_contract_v2',
      ok: policyRel === 'client/runtime/config/production_readiness_closure_policy.json',
      detail: policyRel || policyPath,
    },
    {
      id: 'release_verdict_policy_file_exists_contract',
      ok: fs.existsSync(policyPath),
      detail: policyRel || policyPath,
    },
    {
      id: 'release_verdict_policy_parse_contract',
      ok: policyParsed !== null && typeof policyParsed === 'object' && !Array.isArray(policyParsed),
      detail: policyParsed ? 'ok' : 'parse_or_type_error',
    },
    {
      id: 'release_verdict_gate_registry_path_json_contract',
      ok: gateRegistryPath.endsWith('.json'),
      detail: gateRegistryRel || gateRegistryPath,
    },
    {
      id: 'release_verdict_gate_registry_path_repo_scope_contract',
      ok: gateRegistryInRepo,
      detail: gateRegistryRel || gateRegistryPath,
    },
    {
      id: 'release_verdict_gate_registry_path_expected_contract_v2',
      ok: gateRegistryRel === 'tests/tooling/config/tooling_gate_registry.json',
      detail: gateRegistryRel || gateRegistryPath,
    },
    {
      id: 'release_verdict_gate_registry_file_exists_contract',
      ok: fs.existsSync(gateRegistryPath),
      detail: gateRegistryRel || gateRegistryPath,
    },
    {
      id: 'release_verdict_gate_registry_parse_contract',
      ok: gateRegistryParsed !== null && typeof gateRegistryParsed === 'object' && !Array.isArray(gateRegistryParsed),
      detail: gateRegistryParsed ? 'ok' : 'parse_or_type_error',
    },
    {
      id: 'release_verdict_gate_registry_schema_version_contract',
      ok: cleanText(gateRegistryParsed?.version || '', 40) === '1.0',
      detail: cleanText(gateRegistryParsed?.version || '', 40) || 'missing',
    },
    {
      id: 'release_verdict_gate_registry_gates_object_contract',
      ok:
        gateRegistryParsed?.gates &&
        typeof gateRegistryParsed.gates === 'object' &&
        !Array.isArray(gateRegistryParsed.gates) &&
        gateRegistryGateIds.length > 0,
      detail: `gate_count=${gateRegistryGateIds.length}`,
    },
    {
      id: 'release_verdict_gate_registry_gate_ids_token_contract',
      ok: gateRegistryGateIdsInvalid.length === 0,
      detail:
        gateRegistryGateIdsInvalid.length === 0
          ? 'ok'
          : gateRegistryGateIdsInvalid.join(','),
    },
    {
      id: 'release_verdict_policy_schema_contract',
      ok:
        cleanText(policy?.schema_id || '', 120) === 'production_readiness_closure_policy' &&
        cleanText(policy?.schema_version || '', 40) === '1.0',
      detail: `schema_id=${cleanText(policy?.schema_id || '', 120) || 'missing'};schema_version=${cleanText(policy?.schema_version || '', 40) || 'missing'}`,
    },
    {
      id: 'release_verdict_policy_keyset_contract',
      ok: verdictPolicyKeyset.missing.length === 0 && verdictPolicyKeyset.unexpected.length === 0,
      detail:
        verdictPolicyKeyset.missing.length === 0 && verdictPolicyKeyset.unexpected.length === 0
          ? 'ok'
          : `missing=${verdictPolicyKeyset.missing.join(',') || 'none'};unexpected=${verdictPolicyKeyset.unexpected.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_script_contract',
      ok: cleanText(verdictPolicy?.script || '', 120) === 'ops:release:verdict',
      detail: cleanText(verdictPolicy?.script || '', 120) || 'missing',
    },
    {
      id: 'release_verdict_artifact_path_contract',
      ok:
        cleanText(verdictPolicy?.artifact_path || '', 260) ===
          'core/local/artifacts/release_verdict_current.json' &&
        isCanonicalRelativePathToken(verdictPolicy?.artifact_path, 'core/local/artifacts/', '_current.json'),
      detail: cleanText(verdictPolicy?.artifact_path || '', 260) || 'missing',
    },
    {
      id: 'release_verdict_out_path_matches_policy_artifact_contract_v2',
      ok: outPathMatchesPolicyArtifact,
      detail: `out=${outRel || outPath};policy=${cleanText(verdictPolicy?.artifact_path || '', 500) || 'missing'}`,
    },
    {
      id: 'release_verdict_required_gate_artifacts_object_contract',
      ok:
        requiredGateArtifacts &&
        typeof requiredGateArtifacts === 'object' &&
        !Array.isArray(requiredGateArtifacts) &&
        requiredGateArtifactEntries.length > 0,
      detail: `entries=${requiredGateArtifactEntries.length}`,
    },
    {
      id: 'release_verdict_required_gate_ids_token_contract',
      ok: requiredGateIds.every((gateId) => GATE_ID_TOKEN_REGEX.test(gateId)),
      detail: requiredGateIds.join(','),
    },
    {
      id: 'release_verdict_required_gate_ids_unique_contract',
      ok: requiredGateIdsDuplicate.length === 0,
      detail:
        requiredGateIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(requiredGateIdsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_required_gate_artifact_paths_token_contract',
      ok: requiredGateArtifactPaths.every((relPath) => isCanonicalRelativePathToken(relPath, '', '.json')),
      detail: requiredGateArtifactPaths.join(','),
    },
    {
      id: 'release_verdict_required_gate_artifact_paths_unique_contract',
      ok: requiredGateArtifactPathsDuplicate.length === 0,
      detail:
        requiredGateArtifactPathsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(requiredGateArtifactPathsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_required_gate_baseline_contract',
      ok: requiredGateArtifactMissingBaseline.length === 0,
      detail:
        requiredGateArtifactMissingBaseline.length === 0
          ? 'ok'
          : requiredGateArtifactMissingBaseline.join(','),
    },
    {
      id: 'release_verdict_required_gates_registered_contract',
      ok: requiredGateIdsMissingInRegistry.length === 0,
      detail:
        requiredGateIdsMissingInRegistry.length === 0
          ? 'ok'
          : requiredGateIdsMissingInRegistry.join(','),
    },
    {
      id: 'release_verdict_required_gate_registry_artifact_binding_contract',
      ok: requiredGateRegistryArtifactBindingMismatches.length === 0,
      detail:
        requiredGateRegistryArtifactBindingMismatches.length === 0
          ? 'ok'
          : requiredGateRegistryArtifactBindingMismatches.join(','),
    },
    {
      id: 'release_verdict_verify_profiles_path_json_contract',
      ok: verifyProfilesPath.endsWith('.json'),
      detail: verifyProfilesRel || verifyProfilesPath,
    },
    {
      id: 'release_verdict_verify_profiles_path_repo_scope_contract',
      ok: verifyProfilesInRepo,
      detail: verifyProfilesRel || verifyProfilesPath,
    },
    {
      id: 'release_verdict_verify_profiles_path_expected_contract_v2',
      ok: verifyProfilesRel === 'tests/tooling/config/verify_profiles.json',
      detail: verifyProfilesRel || verifyProfilesPath,
    },
    {
      id: 'release_verdict_verify_profiles_file_exists_contract',
      ok: fs.existsSync(verifyProfilesPath),
      detail: verifyProfilesRel || verifyProfilesPath,
    },
    {
      id: 'release_verdict_verify_profiles_parse_contract',
      ok: verifyProfilesParsed !== null && typeof verifyProfilesParsed === 'object' && !Array.isArray(verifyProfilesParsed),
      detail: verifyProfilesParsed ? 'ok' : 'parse_or_type_error',
    },
    {
      id: 'release_verdict_verify_profiles_version_contract',
      ok: cleanText(verifyProfilesParsed?.version || '', 40) === '1.0',
      detail: cleanText(verifyProfilesParsed?.version || '', 40) || 'missing',
    },
    {
      id: 'release_verdict_verify_profiles_release_profile_present_contract',
      ok: Array.isArray(verifyProfilesParsed?.profiles?.release?.gate_ids) && releaseProfileGateIds.length > 0,
      detail: `release_gate_count=${releaseProfileGateIds.length}`,
    },
    {
      id: 'release_verdict_verify_profiles_runtime_proof_profile_present_contract',
      ok:
        Array.isArray(verifyProfilesParsed?.profiles?.['runtime-proof']?.gate_ids) &&
        runtimeProofProfileGateIds.length > 0,
      detail: `runtime_proof_gate_count=${runtimeProofProfileGateIds.length}`,
    },
    {
      id: 'release_verdict_release_profile_gate_ids_unique_contract',
      ok: releaseProfileGateIdsDuplicate.length === 0,
      detail:
        releaseProfileGateIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(releaseProfileGateIdsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_release_profile_gate_ids_token_contract',
      ok: releaseProfileGateIdsInvalid.length === 0,
      detail:
        releaseProfileGateIdsInvalid.length === 0
          ? 'ok'
          : releaseProfileGateIdsInvalid.join(','),
    },
    {
      id: 'release_verdict_release_profile_gate_ids_registered_contract_v2',
      ok: releaseProfileGateIdsMissingInRegistry.length === 0,
      detail:
        releaseProfileGateIdsMissingInRegistry.length === 0
          ? 'ok'
          : releaseProfileGateIdsMissingInRegistry.join(','),
    },
    {
      id: 'release_verdict_release_profile_required_subset_order_contract_v2',
      ok: releaseProfileRequiredOrderValid,
      detail: `indexes=${releaseProfileRequiredIndexes.join(',')}`,
    },
    {
      id: 'release_verdict_release_profile_required_subset_contract_v2',
      ok: releaseProfileRequiredMissing.length === 0,
      detail:
        releaseProfileRequiredMissing.length === 0
          ? 'ok'
          : releaseProfileRequiredMissing.join(','),
    },
    {
      id: 'release_verdict_release_profile_required_subset_registered_contract_v2',
      ok: releaseProfileRequiredMissingInRegistry.length === 0,
      detail:
        releaseProfileRequiredMissingInRegistry.length === 0
          ? 'ok'
          : releaseProfileRequiredMissingInRegistry.join(','),
    },
    {
      id: 'release_verdict_required_gates_in_release_profile_contract',
      ok: requiredGateIdsMissingInReleaseProfile.length === 0,
      detail:
        requiredGateIdsMissingInReleaseProfile.length === 0
          ? 'ok'
          : requiredGateIdsMissingInReleaseProfile.join(','),
    },
    {
      id: 'release_verdict_runtime_proof_profile_gate_ids_unique_contract_v2',
      ok: runtimeProofProfileGateIdsDuplicate.length === 0,
      detail:
        runtimeProofProfileGateIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(runtimeProofProfileGateIdsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_runtime_proof_profile_gate_ids_token_contract_v2',
      ok: runtimeProofProfileGateIdsInvalid.length === 0,
      detail:
        runtimeProofProfileGateIdsInvalid.length === 0
          ? 'ok'
          : runtimeProofProfileGateIdsInvalid.join(','),
    },
    {
      id: 'release_verdict_runtime_proof_profile_gate_ids_registered_contract_v2',
      ok: runtimeProofProfileGateIdsMissingInRegistry.length === 0,
      detail:
        runtimeProofProfileGateIdsMissingInRegistry.length === 0
          ? 'ok'
          : runtimeProofProfileGateIdsMissingInRegistry.join(','),
    },
    {
      id: 'release_verdict_runtime_proof_subset_in_release_profile_contract_v2',
      ok: runtimeProofMissingInReleaseProfile.length === 0,
      detail:
        runtimeProofMissingInReleaseProfile.length === 0
          ? 'ok'
          : runtimeProofMissingInReleaseProfile.join(','),
    },
    {
      id: 'release_verdict_runtime_proof_relative_order_in_release_profile_contract_v2',
      ok: runtimeProofRelativeOrderInReleaseProfileValid,
      detail: `indexes=${runtimeProofIndexesInReleaseProfile.join(',')}`,
    },
    {
      id: 'release_verdict_runtime_proof_required_subset_order_contract_v2',
      ok: runtimeProofRequiredOrderValid,
      detail: `indexes=${runtimeProofRequiredIndexes.join(',')}`,
    },
    {
      id: 'release_verdict_runtime_proof_required_subset_contract',
      ok: runtimeProofRequiredMissing.length === 0,
      detail:
        runtimeProofRequiredMissing.length === 0
          ? 'ok'
          : runtimeProofRequiredMissing.join(','),
    },
    {
      id: 'release_verdict_policy_required_release_verify_profile_present_contract_v2',
      ok: policyRequiredReleaseVerifyGateIds.length > 0,
      detail: `count=${policyRequiredReleaseVerifyGateIds.length}`,
    },
    {
      id: 'release_verdict_policy_required_release_verify_ids_token_contract_v2',
      ok: policyRequiredReleaseVerifyGateIdsInvalid.length === 0,
      detail:
        policyRequiredReleaseVerifyGateIdsInvalid.length === 0
          ? 'ok'
          : policyRequiredReleaseVerifyGateIdsInvalid.join(','),
    },
    {
      id: 'release_verdict_policy_required_release_verify_ids_unique_contract_v2',
      ok: policyRequiredReleaseVerifyGateIdsDuplicate.length === 0,
      detail:
        policyRequiredReleaseVerifyGateIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(policyRequiredReleaseVerifyGateIdsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_policy_required_release_verify_ids_registered_contract_v2',
      ok: policyRequiredReleaseVerifyGateIdsMissingInRegistry.length === 0,
      detail:
        policyRequiredReleaseVerifyGateIdsMissingInRegistry.length === 0
          ? 'ok'
          : policyRequiredReleaseVerifyGateIdsMissingInRegistry.join(','),
    },
    {
      id: 'release_verdict_policy_required_release_verify_ids_in_release_profile_contract_v2',
      ok: policyRequiredReleaseVerifyGateIdsMissingInReleaseProfile.length === 0,
      detail:
        policyRequiredReleaseVerifyGateIdsMissingInReleaseProfile.length === 0
          ? 'ok'
          : policyRequiredReleaseVerifyGateIdsMissingInReleaseProfile.join(','),
    },
    {
      id: 'release_verdict_policy_required_release_verify_order_in_release_profile_contract_v2',
      ok: policyRequiredReleaseVerifyOrderInReleaseProfileValid,
      detail: `indexes=${policyRequiredReleaseVerifyIndexesInReleaseProfile.join(',')}`,
    },
    {
      id: 'release_verdict_required_gate_count_contract_v2',
      ok: requiredGateIds.length === RELEASE_VERDICT_REQUIRED_GATE_ORDER.length,
      detail: `count=${requiredGateIds.length};expected=${RELEASE_VERDICT_REQUIRED_GATE_ORDER.length}`,
    },
    {
      id: 'release_verdict_required_gate_expected_set_contract_v2',
      ok: requiredGateIdsMissingExpected.length === 0 && requiredGateIdsUnexpected.length === 0,
      detail:
        requiredGateIdsMissingExpected.length === 0 && requiredGateIdsUnexpected.length === 0
          ? 'ok'
          : `missing=${requiredGateIdsMissingExpected.join(',') || 'none'};unexpected=${requiredGateIdsUnexpected.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_required_gate_expected_order_contract_v2',
      ok: requiredGateOrderMatchesExpected,
      detail: requiredGateIds.join(','),
    },
    {
      id: 'release_verdict_required_gate_expected_path_map_contract_v2',
      ok: requiredGatePathMismatches.length === 0,
      detail: requiredGatePathMismatches.length === 0 ? 'ok' : requiredGatePathMismatches.join(','),
    },
    {
      id: 'release_verdict_required_gate_path_scope_allowlist_contract_v2',
      ok: requiredGatePathsOutsideAllowedPrefixes.length === 0,
      detail:
        requiredGatePathsOutsideAllowedPrefixes.length === 0
          ? 'ok'
          : requiredGatePathsOutsideAllowedPrefixes.join(','),
    },
    {
      id: 'release_verdict_required_gate_filename_contract_v2',
      ok: requiredGatePathsInvalidFilename.length === 0,
      detail:
        requiredGatePathsInvalidFilename.length === 0
          ? 'ok'
          : requiredGatePathsInvalidFilename.join(','),
    },
    {
      id: 'release_verdict_required_gate_basename_unique_contract_v2',
      ok: requiredGateArtifactBasenamesDuplicate.length === 0,
      detail:
        requiredGateArtifactBasenamesDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(requiredGateArtifactBasenamesDuplicate)).join(','),
    },
    {
      id: 'release_verdict_required_gate_family_presence_contract_v2',
      ok:
        requiredGateFamilyCounts.ops >= 14 &&
        requiredGateFamilyCounts.chaos >= 1 &&
        requiredGateFamilyCounts.state >= 1 &&
        requiredGateFamilyCounts.release_family >= 5,
      detail: `ops=${requiredGateFamilyCounts.ops};chaos=${requiredGateFamilyCounts.chaos};state=${requiredGateFamilyCounts.state};release_family=${requiredGateFamilyCounts.release_family}`,
    },
    {
      id: 'release_verdict_required_gate_layer2_pair_contract_v2',
      ok: requiredGateLayer2PairPresent,
      detail: `layer2_pair_present=${String(requiredGateLayer2PairPresent)}`,
    },
    {
      id: 'release_verdict_checksum_paths_array_contract',
      ok: Array.isArray(verdictPolicy?.checksum_artifact_paths) && checksumPathsTrimmed.length > 0,
      detail: `count=${checksumPathsTrimmed.length}`,
    },
    {
      id: 'release_verdict_checksum_paths_token_contract',
      ok: checksumPathsTrimmed.every((relPath) => isCanonicalRelativePathToken(relPath, '', '.json')),
      detail: checksumPathsTrimmed.join(','),
    },
    {
      id: 'release_verdict_checksum_paths_unique_subset_contract',
      ok: checksumPathsDuplicate.length === 0 && checksumPathsNotInRequired.length === 0,
      detail:
        checksumPathsDuplicate.length === 0 && checksumPathsNotInRequired.length === 0
          ? 'ok'
          : `duplicates=${Array.from(new Set(checksumPathsDuplicate)).join(',') || 'none'};not_in_required=${Array.from(new Set(checksumPathsNotInRequired)).join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_checksum_count_matches_required_contract_v2',
      ok: checksumPathsTrimmed.length === requiredGateArtifactPaths.length,
      detail: `checksum_count=${checksumPathsTrimmed.length};required_count=${requiredGateArtifactPaths.length}`,
    },
    {
      id: 'release_verdict_checksum_set_matches_required_contract_v2',
      ok: checksumPathsMissingRequired.length === 0 && checksumPathsNotInRequired.length === 0,
      detail:
        checksumPathsMissingRequired.length === 0 && checksumPathsNotInRequired.length === 0
          ? 'ok'
          : `missing_required=${checksumPathsMissingRequired.join(',') || 'none'};unexpected=${checksumPathsNotInRequired.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_checksum_order_matches_required_contract_v2',
      ok: checksumOrderMatchesRequired,
      detail: `order_match=${String(checksumOrderMatchesRequired)}`,
    },
    {
      id: 'release_verdict_checksum_scope_allowlist_contract_v2',
      ok: checksumPathsOutsideAllowedPrefixes.length === 0,
      detail:
        checksumPathsOutsideAllowedPrefixes.length === 0
          ? 'ok'
          : checksumPathsOutsideAllowedPrefixes.join(','),
    },
    {
      id: 'release_verdict_checksum_filename_contract_v2',
      ok: checksumPathsInvalidFilename.length === 0,
      detail: checksumPathsInvalidFilename.length === 0 ? 'ok' : checksumPathsInvalidFilename.join(','),
    },
    {
      id: 'release_verdict_checksum_basename_unique_contract_v2',
      ok: checksumBasenamesDuplicate.length === 0,
      detail:
        checksumBasenamesDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(checksumBasenamesDuplicate)).join(','),
    },
    {
      id: 'release_verdict_checksum_required_baseline_contract_v2',
      ok: checksumRequiredBaselineMissing.length === 0,
      detail:
        checksumRequiredBaselineMissing.length === 0
          ? 'ok'
          : checksumRequiredBaselineMissing.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_gate_coverage_contract',
      ok: rcStepGateIdsDuplicate.length === 0 && rcStepGateIdsMissing.length === 0,
      detail:
        rcStepGateIdsDuplicate.length === 0 && rcStepGateIdsMissing.length === 0
          ? 'ok'
          : `duplicate_step_gate_ids=${Array.from(new Set(rcStepGateIdsDuplicate)).join(',') || 'none'};missing_step_gate_ids=${rcStepGateIdsMissing.join(',') || 'none'}`,
    },
    {
      id: 'release_candidate_rehearsal_steps_array_contract_v2',
      ok: Array.isArray(rcPayload?.steps) && rcSteps.length > 0 && rcStepRowsAreObjects,
      detail: `steps=${rcSteps.length};rows_are_objects=${String(rcStepRowsAreObjects)}`,
    },
    {
      id: 'release_candidate_rehearsal_step_order_token_contract_v3',
      ok: rcStepOrderInvalid.length === 0,
      detail: rcStepOrderInvalid.length === 0 ? 'ok' : rcStepOrderInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_order_unique_contract_v3',
      ok: rcStepOrderDuplicates.length === 0,
      detail:
        rcStepOrderDuplicates.length === 0
          ? 'ok'
          : Array.from(new Set(rcStepOrderDuplicates)).join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_order_contiguous_contract_v3',
      ok: rcStepOrderContiguous,
      detail: `contiguous=${String(rcStepOrderContiguous)};orders=${rcStepOrdersSorted.join(',')}`,
    },
    {
      id: 'release_candidate_rehearsal_failure_rows_array_contract_v3',
      ok: Array.isArray(rcPayload?.failures) && rcFailureRowsAreObjects,
      detail: `failures=${rcFailures.length};rows_are_objects=${String(rcFailureRowsAreObjects)}`,
    },
    {
      id: 'release_candidate_rehearsal_failure_gate_token_contract_v3',
      ok: rcFailureGateIdsInvalid.length === 0,
      detail: rcFailureGateIdsInvalid.length === 0 ? 'ok' : rcFailureGateIdsInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_failure_gate_unique_contract_v3',
      ok: rcFailureGateIdsDuplicate.length === 0,
      detail:
        rcFailureGateIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(rcFailureGateIdsDuplicate)).join(','),
    },
    {
      id: 'release_candidate_rehearsal_failure_gate_subset_steps_contract_v3',
      ok: rcFailureGateIdsMissingInSteps.length === 0,
      detail:
        rcFailureGateIdsMissingInSteps.length === 0
          ? 'ok'
          : rcFailureGateIdsMissingInSteps.join(','),
    },
    {
      id: 'release_candidate_rehearsal_failure_order_refers_to_step_contract_v3',
      ok: rcFailureOrderMissingInSteps.length === 0,
      detail:
        rcFailureOrderMissingInSteps.length === 0
          ? 'ok'
          : rcFailureOrderMissingInSteps.join(','),
    },
    {
      id: 'release_candidate_rehearsal_summary_step_count_contract_v3',
      ok: Number.isInteger(rcSummaryStepCount) && rcSummaryStepCount === rcSteps.length,
      detail: `summary_step_count=${rcSummaryStepCount};actual=${rcSteps.length}`,
    },
    {
      id: 'release_candidate_rehearsal_summary_failed_count_contract_v3',
      ok: Number.isInteger(rcSummaryFailedCount) && rcSummaryFailedCount === rcFailures.length,
      detail: `summary_failed_count=${rcSummaryFailedCount};actual=${rcFailures.length}`,
    },
    {
      id: 'release_candidate_rehearsal_summary_required_step_count_contract_v3',
      ok:
        Number.isInteger(rcSummaryRequiredStepCount) &&
        rcSummaryRequiredStepCount === rcPolicyRequiredStepGateIds.length,
      detail: `summary_required_step_count=${rcSummaryRequiredStepCount};policy_required=${rcPolicyRequiredStepGateIds.length}`,
    },
    {
      id: 'release_candidate_rehearsal_summary_required_satisfied_consistency_contract_v3',
      ok: rcSummaryRequiredStepsSatisfied === rcDerivedRequiredStepsSatisfied,
      detail: `summary=${String(rcSummaryRequiredStepsSatisfied)};derived=${String(rcDerivedRequiredStepsSatisfied)}`,
    },
    {
      id: 'release_candidate_rehearsal_summary_candidate_ready_consistency_contract_v3',
      ok: !rcSummaryCandidateReady || (rcSummaryRequiredStepsSatisfied && rcSummaryFailedCount === 0),
      detail: `candidate_ready=${String(rcSummaryCandidateReady)};required_steps=${String(rcSummaryRequiredStepsSatisfied)};failed_count=${rcSummaryFailedCount}`,
    },
    {
      id: 'release_verdict_rc_policy_required_steps_present_contract_v2',
      ok: rcPolicyRequiredStepGateIds.length > 0,
      detail: `count=${rcPolicyRequiredStepGateIds.length}`,
    },
    {
      id: 'release_verdict_rc_policy_required_step_ids_token_contract_v2',
      ok: rcPolicyRequiredStepGateIdsInvalid.length === 0,
      detail:
        rcPolicyRequiredStepGateIdsInvalid.length === 0
          ? 'ok'
          : rcPolicyRequiredStepGateIdsInvalid.join(','),
    },
    {
      id: 'release_verdict_rc_policy_required_step_ids_unique_contract_v2',
      ok: rcPolicyRequiredStepGateIdsDuplicate.length === 0,
      detail:
        rcPolicyRequiredStepGateIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(rcPolicyRequiredStepGateIdsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_rc_policy_required_step_ids_registered_contract_v2',
      ok: rcPolicyRequiredStepGateIdsMissingInRegistry.length === 0,
      detail:
        rcPolicyRequiredStepGateIdsMissingInRegistry.length === 0
          ? 'ok'
          : rcPolicyRequiredStepGateIdsMissingInRegistry.join(','),
    },
    {
      id: 'release_verdict_rc_policy_required_step_ids_in_release_profile_contract_v2',
      ok: rcPolicyRequiredStepGateIdsMissingInReleaseProfile.length === 0,
      detail:
        rcPolicyRequiredStepGateIdsMissingInReleaseProfile.length === 0
          ? 'ok'
          : rcPolicyRequiredStepGateIdsMissingInReleaseProfile.join(','),
    },
    {
      id: 'release_verdict_rc_policy_covers_required_gate_artifacts_contract_v2',
      ok: rcPolicyMissingRequiredGateArtifacts.length === 0,
      detail:
        rcPolicyMissingRequiredGateArtifacts.length === 0
          ? 'ok'
          : rcPolicyMissingRequiredGateArtifacts.join(','),
    },
    {
      id: 'release_verdict_rc_policy_extra_steps_expected_contract_v2',
      ok: rcPolicyExtraStepSetMatches,
      detail: `extras=${rcPolicyExtraStepGateIds.join(',') || 'none'};expected=${rcPolicyExtraStepExpected.join(',')}`,
    },
    {
      id: 'release_candidate_rehearsal_step_gate_token_contract_v2',
      ok: rcStepGateIdsInvalidToken.length === 0,
      detail:
        rcStepGateIdsInvalidToken.length === 0
          ? 'ok'
          : rcStepGateIdsInvalidToken.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_ok_boolean_contract_v2',
      ok: rcStepMissingOkBooleanCount === 0,
      detail: `missing_ok_boolean_count=${rcStepMissingOkBooleanCount}`,
    },
    {
      id: 'release_candidate_rehearsal_step_gate_expected_contract_v2',
      ok: rcStepGateIdsUnexpected.length === 0,
      detail:
        rcStepGateIdsUnexpected.length === 0
          ? 'ok'
          : rcStepGateIdsUnexpected.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_ids_in_rc_policy_contract_v2',
      ok: rcStepGateIdsMissingInPolicy.length === 0,
      detail:
        rcStepGateIdsMissingInPolicy.length === 0
          ? 'ok'
          : rcStepGateIdsMissingInPolicy.join(','),
    },
    {
      id: 'release_candidate_rehearsal_policy_steps_present_in_rehearsal_contract_v2',
      ok: rcPolicyRequiredStepGateIdsMissingInRcSteps.length === 0,
      detail:
        rcPolicyRequiredStepGateIdsMissingInRcSteps.length === 0
          ? 'ok'
          : rcPolicyRequiredStepGateIdsMissingInRcSteps.join(','),
    },
    {
      id: 'release_candidate_rehearsal_policy_step_order_contract_v2',
      ok: rcPolicyRequiredStepOrderInRcValid,
      detail: `indexes=${rcPolicyRequiredStepIndexesInRc.join(',')}`,
    },
    {
      id: 'release_verdict_policy_required_files_baseline_contract_v3',
      ok: policyRequiredFilesMissingBaseline.length === 0,
      detail:
        policyRequiredFilesMissingBaseline.length === 0
          ? 'ok'
          : policyRequiredFilesMissingBaseline.join(','),
    },
    {
      id: 'release_verdict_policy_required_files_unique_contract_v3',
      ok: policyRequiredFilesDuplicate.length === 0,
      detail:
        policyRequiredFilesDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(policyRequiredFilesDuplicate)).join(','),
    },
    {
      id: 'release_verdict_policy_required_files_token_contract_v3',
      ok: policyRequiredFilesInvalid.length === 0,
      detail:
        policyRequiredFilesInvalid.length === 0
          ? 'ok'
          : policyRequiredFilesInvalid.join(','),
    },
    {
      id: 'release_verdict_policy_required_package_scripts_baseline_contract_v3',
      ok: policyRequiredPackageScriptsMissingBaseline.length === 0,
      detail:
        policyRequiredPackageScriptsMissingBaseline.length === 0
          ? 'ok'
          : policyRequiredPackageScriptsMissingBaseline.join(','),
    },
    {
      id: 'release_verdict_policy_required_package_scripts_unique_trim_contract_v3',
      ok:
        policyRequiredPackageScriptsDuplicate.length === 0 &&
        policyRequiredPackageScriptsInvalid.length === 0,
      detail:
        policyRequiredPackageScriptsDuplicate.length === 0 &&
        policyRequiredPackageScriptsInvalid.length === 0
          ? 'ok'
          : `duplicate=${Array.from(new Set(policyRequiredPackageScriptsDuplicate)).join(',') || 'none'};invalid=${policyRequiredPackageScriptsInvalid.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_policy_required_ci_invocations_baseline_contract_v3',
      ok: policyRequiredCiInvocationsMissingBaseline.length === 0,
      detail:
        policyRequiredCiInvocationsMissingBaseline.length === 0
          ? 'ok'
          : policyRequiredCiInvocationsMissingBaseline.join(','),
    },
    {
      id: 'release_verdict_policy_required_ci_invocations_unique_trim_contract_v3',
      ok:
        policyRequiredCiInvocationsDuplicate.length === 0 &&
        policyRequiredCiInvocationsInvalid.length === 0,
      detail:
        policyRequiredCiInvocationsDuplicate.length === 0 &&
        policyRequiredCiInvocationsInvalid.length === 0
          ? 'ok'
          : `duplicate=${Array.from(new Set(policyRequiredCiInvocationsDuplicate)).join(',') || 'none'};invalid=${policyRequiredCiInvocationsInvalid.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_required_gate_registry_rows_present_contract_v4',
      ok: requiredGateRegistryMissingRows.length === 0,
      detail:
        requiredGateRegistryMissingRows.length === 0
          ? 'ok'
          : requiredGateRegistryMissingRows.join(','),
    },
    {
      id: 'release_verdict_required_gate_registry_owner_contract_v4',
      ok: requiredGateRegistryOwnerInvalid.length === 0,
      detail:
        requiredGateRegistryOwnerInvalid.length === 0
          ? 'ok'
          : requiredGateRegistryOwnerInvalid.join(','),
    },
    {
      id: 'release_verdict_required_gate_registry_description_contract_v4',
      ok: requiredGateRegistryDescriptionInvalid.length === 0,
      detail:
        requiredGateRegistryDescriptionInvalid.length === 0
          ? 'ok'
          : requiredGateRegistryDescriptionInvalid.join(','),
    },
    {
      id: 'release_verdict_required_gate_registry_timeout_contract_v4',
      ok: requiredGateRegistryTimeoutInvalid.length === 0,
      detail:
        requiredGateRegistryTimeoutInvalid.length === 0
          ? 'ok'
          : requiredGateRegistryTimeoutInvalid.join(','),
    },
    {
      id: 'release_verdict_required_gate_registry_timeout_env_contract_v4',
      ok: requiredGateRegistryTimeoutEnvInvalid.length === 0,
      detail:
        requiredGateRegistryTimeoutEnvInvalid.length === 0
          ? 'ok'
          : requiredGateRegistryTimeoutEnvInvalid.join(','),
    },
    {
      id: 'release_verdict_required_gate_registry_selector_contract_v4',
      ok: requiredGateRegistrySelectorInvalid.length === 0,
      detail:
        requiredGateRegistrySelectorInvalid.length === 0
          ? 'ok'
          : requiredGateRegistrySelectorInvalid.join(','),
    },
    {
      id: 'release_verdict_required_gate_registry_command_shape_contract_v4',
      ok: requiredGateRegistryCommandShapeInvalid.length === 0,
      detail:
        requiredGateRegistryCommandShapeInvalid.length === 0
          ? 'ok'
          : requiredGateRegistryCommandShapeInvalid.join(','),
    },
    {
      id: 'release_verdict_required_gate_registry_script_shape_contract_v4',
      ok: requiredGateRegistryScriptShapeInvalid.length === 0,
      detail:
        requiredGateRegistryScriptShapeInvalid.length === 0
          ? 'ok'
          : requiredGateRegistryScriptShapeInvalid.join(','),
    },
    {
      id: 'release_verdict_required_gate_registry_artifact_array_contract_v4',
      ok: requiredGateRegistryArtifactArrayInvalid.length === 0,
      detail:
        requiredGateRegistryArtifactArrayInvalid.length === 0
          ? 'ok'
          : requiredGateRegistryArtifactArrayInvalid.join(','),
    },
    {
      id: 'release_verdict_required_gate_registry_artifact_token_contract_v4',
      ok: requiredGateRegistryArtifactTokenInvalid.length === 0,
      detail:
        requiredGateRegistryArtifactTokenInvalid.length === 0
          ? 'ok'
          : requiredGateRegistryArtifactTokenInvalid.join(','),
    },
    {
      id: 'release_verdict_required_gate_registry_artifact_unique_contract_v4',
      ok: requiredGateRegistryArtifactDuplicateInvalid.length === 0,
      detail:
        requiredGateRegistryArtifactDuplicateInvalid.length === 0
          ? 'ok'
          : requiredGateRegistryArtifactDuplicateInvalid.join(','),
    },
    {
      id: 'release_verdict_release_profile_registry_rows_present_contract_v4',
      ok: releaseProfileRegistryMissingRows.length === 0,
      detail:
        releaseProfileRegistryMissingRows.length === 0
          ? 'ok'
          : releaseProfileRegistryMissingRows.join(','),
    },
    {
      id: 'release_verdict_release_profile_registry_owner_contract_v4',
      ok: releaseProfileRegistryOwnerInvalid.length === 0,
      detail:
        releaseProfileRegistryOwnerInvalid.length === 0
          ? 'ok'
          : releaseProfileRegistryOwnerInvalid.join(','),
    },
    {
      id: 'release_verdict_release_profile_registry_description_contract_v4',
      ok: releaseProfileRegistryDescriptionInvalid.length === 0,
      detail:
        releaseProfileRegistryDescriptionInvalid.length === 0
          ? 'ok'
          : releaseProfileRegistryDescriptionInvalid.join(','),
    },
    {
      id: 'release_verdict_release_profile_registry_timeout_contract_v4',
      ok: releaseProfileRegistryTimeoutInvalid.length === 0,
      detail:
        releaseProfileRegistryTimeoutInvalid.length === 0
          ? 'ok'
          : releaseProfileRegistryTimeoutInvalid.join(','),
    },
    {
      id: 'release_verdict_release_profile_registry_selector_contract_v4',
      ok: releaseProfileRegistrySelectorInvalid.length === 0,
      detail:
        releaseProfileRegistrySelectorInvalid.length === 0
          ? 'ok'
          : releaseProfileRegistrySelectorInvalid.join(','),
    },
    {
      id: 'release_verdict_runtime_proof_registry_rows_present_contract_v4',
      ok: runtimeProofRegistryMissingRows.length === 0,
      detail:
        runtimeProofRegistryMissingRows.length === 0
          ? 'ok'
          : runtimeProofRegistryMissingRows.join(','),
    },
    {
      id: 'release_verdict_runtime_proof_registry_owner_contract_v4',
      ok: runtimeProofRegistryOwnerInvalid.length === 0,
      detail:
        runtimeProofRegistryOwnerInvalid.length === 0
          ? 'ok'
          : runtimeProofRegistryOwnerInvalid.join(','),
    },
    {
      id: 'release_verdict_runtime_proof_registry_timeout_contract_v4',
      ok: runtimeProofRegistryTimeoutInvalid.length === 0,
      detail:
        runtimeProofRegistryTimeoutInvalid.length === 0
          ? 'ok'
          : runtimeProofRegistryTimeoutInvalid.join(','),
    },
    {
      id: 'release_verdict_runtime_proof_registry_selector_contract_v4',
      ok: runtimeProofRegistrySelectorInvalid.length === 0,
      detail:
        runtimeProofRegistrySelectorInvalid.length === 0
          ? 'ok'
          : runtimeProofRegistrySelectorInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_duration_contract_v5',
      ok: rcStepDurationInvalid.length === 0,
      detail: rcStepDurationInvalid.length === 0 ? 'ok' : rcStepDurationInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_exit_code_contract_v5',
      ok: rcStepExitCodeInvalid.length === 0,
      detail: rcStepExitCodeInvalid.length === 0 ? 'ok' : rcStepExitCodeInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_failure_message_contract_v5',
      ok: rcStepFailureMessageInvalid.length === 0,
      detail: rcStepFailureMessageInvalid.length === 0 ? 'ok' : rcStepFailureMessageInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_artifact_paths_array_contract_v5',
      ok: rcStepArtifactPathsArrayInvalid.length === 0,
      detail: rcStepArtifactPathsArrayInvalid.length === 0 ? 'ok' : rcStepArtifactPathsArrayInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_artifact_paths_token_contract_v5',
      ok: rcStepArtifactPathsTokenInvalid.length === 0,
      detail: rcStepArtifactPathsTokenInvalid.length === 0 ? 'ok' : rcStepArtifactPathsTokenInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_artifact_paths_unique_contract_v5',
      ok: rcStepArtifactPathsDuplicateInvalid.length === 0,
      detail: rcStepArtifactPathsDuplicateInvalid.length === 0 ? 'ok' : rcStepArtifactPathsDuplicateInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_failed_ids_array_contract_v5',
      ok: rcStepFailedIdsArrayInvalid.length === 0,
      detail: rcStepFailedIdsArrayInvalid.length === 0 ? 'ok' : rcStepFailedIdsArrayInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_failed_ids_token_contract_v5',
      ok: rcStepFailedIdsTokenInvalid.length === 0,
      detail: rcStepFailedIdsTokenInvalid.length === 0 ? 'ok' : rcStepFailedIdsTokenInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_failed_ids_unique_contract_v5',
      ok: rcStepFailedIdsDuplicateInvalid.length === 0,
      detail: rcStepFailedIdsDuplicateInvalid.length === 0 ? 'ok' : rcStepFailedIdsDuplicateInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_degraded_flags_array_contract_v5',
      ok: rcStepDegradedFlagsArrayInvalid.length === 0,
      detail: rcStepDegradedFlagsArrayInvalid.length === 0 ? 'ok' : rcStepDegradedFlagsArrayInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_degraded_flags_token_contract_v5',
      ok: rcStepDegradedFlagsTokenInvalid.length === 0,
      detail: rcStepDegradedFlagsTokenInvalid.length === 0 ? 'ok' : rcStepDegradedFlagsTokenInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_degraded_flags_unique_contract_v5',
      ok: rcStepDegradedFlagsDuplicateInvalid.length === 0,
      detail: rcStepDegradedFlagsDuplicateInvalid.length === 0 ? 'ok' : rcStepDegradedFlagsDuplicateInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_failure_ok_false_contract_v5',
      ok: rcFailureOkFalseInvalid.length === 0,
      detail: rcFailureOkFalseInvalid.length === 0 ? 'ok' : rcFailureOkFalseInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_failure_exit_code_contract_v5',
      ok: rcFailureExitCodeInvalid.length === 0,
      detail: rcFailureExitCodeInvalid.length === 0 ? 'ok' : rcFailureExitCodeInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_failure_artifact_paths_array_contract_v5',
      ok: rcFailureArtifactPathsArrayInvalid.length === 0,
      detail: rcFailureArtifactPathsArrayInvalid.length === 0 ? 'ok' : rcFailureArtifactPathsArrayInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_failure_artifact_paths_token_contract_v5',
      ok: rcFailureArtifactPathsTokenInvalid.length === 0,
      detail: rcFailureArtifactPathsTokenInvalid.length === 0 ? 'ok' : rcFailureArtifactPathsTokenInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_failure_artifact_paths_unique_contract_v5',
      ok: rcFailureArtifactPathsDuplicateInvalid.length === 0,
      detail: rcFailureArtifactPathsDuplicateInvalid.length === 0 ? 'ok' : rcFailureArtifactPathsDuplicateInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_failure_orders_failed_step_subset_contract_v5',
      ok: rcFailureOrdersMissingInFailedSteps.length === 0,
      detail:
        rcFailureOrdersMissingInFailedSteps.length === 0
          ? 'ok'
          : rcFailureOrdersMissingInFailedSteps.join(','),
    },
    {
      id: 'release_candidate_rehearsal_failure_gate_ids_failed_step_subset_contract_v5',
      ok: rcFailureGateIdsMissingInFailedSteps.length === 0,
      detail:
        rcFailureGateIdsMissingInFailedSteps.length === 0
          ? 'ok'
          : rcFailureGateIdsMissingInFailedSteps.join(','),
    },
    {
      id: 'release_candidate_rehearsal_top_level_ok_consistency_contract_v5',
      ok:
        (rcPayload?.ok === true && rcSummaryFailedCount === 0 && rcFailures.length === 0 && rcSummaryCandidateReady) ||
        (rcPayload?.ok === false),
      detail: `ok=${String(rcPayload?.ok === true)};summary_failed=${rcSummaryFailedCount};failures=${rcFailures.length};candidate_ready=${String(rcSummaryCandidateReady)}`,
    },
    {
      id: 'release_candidate_rehearsal_recovery_section_object_contract_v6',
      ok: rcRecoveryIsObject,
      detail: `object=${String(rcRecoveryIsObject)}`,
    },
    {
      id: 'release_candidate_rehearsal_chaos_section_object_contract_v6',
      ok: rcChaosIsObject,
      detail: `object=${String(rcChaosIsObject)}`,
    },
    {
      id: 'release_candidate_rehearsal_replay_section_object_contract_v6',
      ok: rcReplayIsObject,
      detail: `object=${String(rcReplayIsObject)}`,
    },
    {
      id: 'release_candidate_rehearsal_topology_section_object_contract_v6',
      ok: rcTopologyIsObject,
      detail: `object=${String(rcTopologyIsObject)}`,
    },
    {
      id: 'release_candidate_rehearsal_client_boundary_section_object_contract_v6',
      ok: rcClientBoundaryIsObject,
      detail: `object=${String(rcClientBoundaryIsObject)}`,
    },
    {
      id: 'release_candidate_rehearsal_hidden_state_section_object_contract_v6',
      ok: rcHiddenStateIsObject,
      detail: `object=${String(rcHiddenStateIsObject)}`,
    },
    {
      id: 'release_candidate_rehearsal_layer2_parity_section_object_contract_v6',
      ok: rcLayer2ParityIsObject,
      detail: `object=${String(rcLayer2ParityIsObject)}`,
    },
    {
      id: 'release_candidate_rehearsal_layer2_receipt_replay_section_object_contract_v6',
      ok: rcLayer2ReceiptReplayIsObject,
      detail: `object=${String(rcLayer2ReceiptReplayIsObject)}`,
    },
    {
      id: 'release_candidate_rehearsal_trusted_core_section_object_contract_v6',
      ok: rcTrustedCoreIsObject,
      detail: `object=${String(rcTrustedCoreIsObject)}`,
    },
    {
      id: 'release_candidate_rehearsal_proof_pack_section_object_contract_v6',
      ok: rcProofPackIsObject,
      detail: `object=${String(rcProofPackIsObject)}`,
    },
    {
      id: 'release_candidate_rehearsal_recovery_section_semantic_contract_v6',
      ok: rcRecoverySemanticValid,
      detail: `ok_boolean=${String(typeof rcRecovery?.ok === 'boolean')};gate_state=${rcRecoveryGateState || 'missing'}`,
    },
    {
      id: 'release_candidate_rehearsal_chaos_section_semantic_contract_v6',
      ok: rcChaosSemanticValid,
      detail: `ok_boolean=${String(typeof rcChaos?.ok === 'boolean')};payload_type=${rcChaosPayloadType || 'missing'}`,
    },
    {
      id: 'release_candidate_rehearsal_replay_section_semantic_contract_v6',
      ok: rcReplaySemanticValid,
      detail: `ok_boolean=${String(typeof rcReplay?.ok === 'boolean')};payload_type=${rcReplayPayloadType || 'missing'}`,
    },
    {
      id: 'release_candidate_rehearsal_topology_degraded_flags_contract_v6',
      ok: rcTopologyDegradedFlagsInvalid.length === 0 && rcTopologyDegradedFlagsDuplicate.length === 0,
      detail:
        rcTopologyDegradedFlagsInvalid.length === 0 && rcTopologyDegradedFlagsDuplicate.length === 0
          ? 'ok'
          : `invalid=${rcTopologyDegradedFlagsInvalid.join(',') || 'none'};duplicate=${Array.from(new Set(rcTopologyDegradedFlagsDuplicate)).join(',') || 'none'}`,
    },
    {
      id: 'release_candidate_rehearsal_client_boundary_failed_ids_contract_v6',
      ok: rcClientBoundaryFailedIdsInvalid.length === 0 && rcClientBoundaryFailedIdsDuplicate.length === 0,
      detail:
        rcClientBoundaryFailedIdsInvalid.length === 0 && rcClientBoundaryFailedIdsDuplicate.length === 0
          ? 'ok'
          : `invalid=${rcClientBoundaryFailedIdsInvalid.join(',') || 'none'};duplicate=${Array.from(new Set(rcClientBoundaryFailedIdsDuplicate)).join(',') || 'none'}`,
    },
    {
      id: 'release_candidate_rehearsal_client_boundary_consistency_contract_v6',
      ok: rcClientBoundaryConsistency,
      detail: `ok=${String(rcClientBoundary?.ok === true)};failed_ids=${rcClientBoundaryFailedIds.length}`,
    },
    {
      id: 'release_candidate_rehearsal_hidden_state_consistency_contract_v6',
      ok: rcHiddenStateConsistency,
      detail: `ok=${String(rcHiddenState?.ok === true)};failure_len=${rcHiddenStateFailure.length}`,
    },
    {
      id: 'release_candidate_rehearsal_layer2_parity_artifact_paths_contract_v6',
      ok: rcLayer2ParityArtifactPathsInvalid.length === 0 && rcLayer2ParityArtifactPathsDuplicate.length === 0,
      detail:
        rcLayer2ParityArtifactPathsInvalid.length === 0 && rcLayer2ParityArtifactPathsDuplicate.length === 0
          ? 'ok'
          : `invalid=${rcLayer2ParityArtifactPathsInvalid.join(',') || 'none'};duplicate=${Array.from(new Set(rcLayer2ParityArtifactPathsDuplicate)).join(',') || 'none'}`,
    },
    {
      id: 'release_candidate_rehearsal_layer2_receipt_replay_artifact_paths_contract_v6',
      ok:
        rcLayer2ReceiptReplayArtifactPathsInvalid.length === 0 &&
        rcLayer2ReceiptReplayArtifactPathsDuplicate.length === 0,
      detail:
        rcLayer2ReceiptReplayArtifactPathsInvalid.length === 0 &&
        rcLayer2ReceiptReplayArtifactPathsDuplicate.length === 0
          ? 'ok'
          : `invalid=${rcLayer2ReceiptReplayArtifactPathsInvalid.join(',') || 'none'};duplicate=${Array.from(new Set(rcLayer2ReceiptReplayArtifactPathsDuplicate)).join(',') || 'none'}`,
    },
    {
      id: 'release_candidate_rehearsal_trusted_core_proof_pack_semantic_contract_v6',
      ok: rcTrustedCoreProofPackSemanticValid,
      detail: `trusted_core_ok=${String(rcTrustedCore?.ok === true)};trusted_core_paths=${rcTrustedCoreArtifactPaths.length};proof_pack_ok=${String(rcProofPack?.ok === true)};proof_pack_paths=${rcProofPackArtifactPaths.length}`,
    },
    {
      id: 'release_candidate_rehearsal_payload_type_contract_v7',
      ok: rcPayloadType === 'release_candidate_dress_rehearsal',
      detail: rcPayloadType || 'missing',
    },
    {
      id: 'release_candidate_rehearsal_generated_at_timestamp_contract_v7',
      ok: rcPayloadGeneratedAtValid,
      detail: rcPayloadGeneratedAtRaw || 'missing',
    },
    {
      id: 'release_candidate_rehearsal_inputs_object_contract_v7',
      ok: rcInputsIsObject,
      detail: `object=${String(rcInputsIsObject)}`,
    },
    {
      id: 'release_candidate_rehearsal_input_registry_path_contract_v7',
      ok: rcInputRegistryPathValid,
      detail: rcInputRegistryPath || 'missing',
    },
    {
      id: 'release_candidate_rehearsal_input_activate_hardening_window_contract_v7',
      ok: rcInputActivateHardeningWindowValid,
      detail: `boolean=${String(rcInputActivateHardeningWindowValid)}`,
    },
    {
      id: 'release_candidate_rehearsal_input_required_step_ids_token_contract_v7',
      ok: rcInputRequiredStepGateIdsInvalid.length === 0,
      detail: rcInputRequiredStepGateIdsInvalid.length === 0 ? 'ok' : rcInputRequiredStepGateIdsInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_input_required_step_ids_unique_contract_v7',
      ok: rcInputRequiredStepGateIdsDuplicate.length === 0,
      detail:
        rcInputRequiredStepGateIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(rcInputRequiredStepGateIdsDuplicate)).join(','),
    },
    {
      id: 'release_candidate_rehearsal_input_required_step_set_matches_policy_contract_v7',
      ok: rcInputRequiredStepSetMatchesPolicy,
      detail: `input_count=${rcInputRequiredStepGateIds.length};policy_count=${rcPolicyRequiredStepGateIds.length}`,
    },
    {
      id: 'release_candidate_rehearsal_input_required_step_order_matches_policy_contract_v7',
      ok: rcInputRequiredStepOrderMatchesPolicy,
      detail: `order_match=${String(rcInputRequiredStepOrderMatchesPolicy)}`,
    },
    {
      id: 'release_candidate_rehearsal_summary_counts_semantic_contract_v7',
      ok: rcSummaryCountsSemanticValid,
      detail: `step_count=${rcSummaryStepCount};failed_count=${rcSummaryFailedCount};required_step_count=${rcSummaryRequiredStepCount}`,
    },
    {
      id: 'release_candidate_rehearsal_summary_failed_count_matches_failed_steps_contract_v7',
      ok: rcSummaryFailedMatchesFailedSteps,
      detail: `summary_failed=${rcSummaryFailedCount};failed_steps=${rcFailedStepOrders.length}`,
    },
    {
      id: 'release_candidate_rehearsal_failure_duration_contract_v7',
      ok: rcFailureDurationInvalid.length === 0,
      detail: rcFailureDurationInvalid.length === 0 ? 'ok' : rcFailureDurationInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_failure_failure_message_contract_v7',
      ok: rcFailureFailureMessageInvalid.length === 0,
      detail: rcFailureFailureMessageInvalid.length === 0 ? 'ok' : rcFailureFailureMessageInvalid.join(','),
    },
    {
      id: 'release_candidate_rehearsal_failure_failed_ids_contract_v7',
      ok:
        rcFailureFailedIdsArrayInvalid.length === 0 &&
        rcFailureFailedIdsTokenInvalid.length === 0 &&
        rcFailureFailedIdsDuplicateInvalid.length === 0,
      detail:
        rcFailureFailedIdsArrayInvalid.length === 0 &&
        rcFailureFailedIdsTokenInvalid.length === 0 &&
        rcFailureFailedIdsDuplicateInvalid.length === 0
          ? 'ok'
          : `array=${rcFailureFailedIdsArrayInvalid.join(',') || 'none'};token=${rcFailureFailedIdsTokenInvalid.join(',') || 'none'};duplicate=${rcFailureFailedIdsDuplicateInvalid.join(',') || 'none'}`,
    },
    {
      id: 'release_candidate_rehearsal_failure_degraded_flags_contract_v7',
      ok:
        rcFailureDegradedFlagsArrayInvalid.length === 0 &&
        rcFailureDegradedFlagsTokenInvalid.length === 0 &&
        rcFailureDegradedFlagsDuplicateInvalid.length === 0,
      detail:
        rcFailureDegradedFlagsArrayInvalid.length === 0 &&
        rcFailureDegradedFlagsTokenInvalid.length === 0 &&
        rcFailureDegradedFlagsDuplicateInvalid.length === 0
          ? 'ok'
          : `array=${rcFailureDegradedFlagsArrayInvalid.join(',') || 'none'};token=${rcFailureDegradedFlagsTokenInvalid.join(',') || 'none'};duplicate=${rcFailureDegradedFlagsDuplicateInvalid.join(',') || 'none'}`,
    },
    {
      id: 'release_candidate_rehearsal_top_artifact_paths_array_contract_v7',
      ok: Array.isArray(rcPayload?.artifact_paths) && rcTopArtifactPaths.length > 0,
      detail: `count=${rcTopArtifactPaths.length}`,
    },
    {
      id: 'release_candidate_rehearsal_top_artifact_paths_token_unique_contract_v7',
      ok: rcTopArtifactPathsInvalid.length === 0 && rcTopArtifactPathsDuplicate.length === 0,
      detail:
        rcTopArtifactPathsInvalid.length === 0 && rcTopArtifactPathsDuplicate.length === 0
          ? 'ok'
          : `invalid=${rcTopArtifactPathsInvalid.join(',') || 'none'};duplicate=${Array.from(new Set(rcTopArtifactPathsDuplicate)).join(',') || 'none'}`,
    },
    {
      id: 'release_candidate_rehearsal_top_artifact_paths_required_subset_contract_v7',
      ok: rcTopArtifactPathsMissingRequired.length === 0,
      detail:
        rcTopArtifactPathsMissingRequired.length === 0
          ? 'ok'
          : rcTopArtifactPathsMissingRequired.join(','),
    },
    {
      id: 'release_candidate_rehearsal_failed_step_orders_present_in_failures_contract_v7',
      ok: rcFailedStepOrdersMissingInFailures.length === 0,
      detail:
        rcFailedStepOrdersMissingInFailures.length === 0
          ? 'ok'
          : rcFailedStepOrdersMissingInFailures.join(','),
    },
    {
      id: 'release_candidate_rehearsal_failed_step_gate_ids_present_in_failures_contract_v7',
      ok: rcFailedStepGateIdsMissingInFailures.length === 0,
      detail:
        rcFailedStepGateIdsMissingInFailures.length === 0
          ? 'ok'
          : rcFailedStepGateIdsMissingInFailures.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_order_matches_array_index_contract_v8',
      ok: rcStepOrderArrayMismatch.length === 0,
      detail: rcStepOrderArrayMismatch.length === 0 ? 'ok' : rcStepOrderArrayMismatch.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_rows_sorted_by_order_contract_v8',
      ok: rcStepRowsSortedByOrder,
      detail: `sorted=${String(rcStepRowsSortedByOrder)}`,
    },
    {
      id: 'release_candidate_rehearsal_failure_order_unique_contract_v8',
      ok: rcFailureOrderDuplicates.length === 0,
      detail:
        rcFailureOrderDuplicates.length === 0
          ? 'ok'
          : Array.from(new Set(rcFailureOrderDuplicates)).join(','),
    },
    {
      id: 'release_candidate_rehearsal_failure_rows_sorted_by_order_contract_v8',
      ok: rcFailureRowsSortedByOrder,
      detail: `sorted=${String(rcFailureRowsSortedByOrder)}`,
    },
    {
      id: 'release_candidate_rehearsal_failure_order_gate_match_step_contract_v8',
      ok: rcFailureOrderGateMismatch.length === 0,
      detail: rcFailureOrderGateMismatch.length === 0 ? 'ok' : rcFailureOrderGateMismatch.join(','),
    },
    {
      id: 'release_candidate_rehearsal_failure_order_failed_step_state_contract_v8',
      ok: rcFailureOrderFailedStateMismatch.length === 0,
      detail:
        rcFailureOrderFailedStateMismatch.length === 0
          ? 'ok'
          : rcFailureOrderFailedStateMismatch.join(','),
    },
    {
      id: 'release_candidate_rehearsal_failure_row_count_matches_failed_steps_contract_v8',
      ok: rcFailures.length === rcFailedStepOrders.length,
      detail: `failures=${rcFailures.length};failed_steps=${rcFailedStepOrders.length}`,
    },
    {
      id: 'release_candidate_rehearsal_step_artifact_paths_subset_top_contract_v8',
      ok: rcStepArtifactPathsMissingFromTop.length === 0,
      detail:
        rcStepArtifactPathsMissingFromTop.length === 0
          ? 'ok'
          : rcStepArtifactPathsMissingFromTop.join(','),
    },
    {
      id: 'release_candidate_rehearsal_failure_artifact_paths_subset_top_contract_v8',
      ok: rcFailureArtifactPathsMissingFromTop.length === 0,
      detail:
        rcFailureArtifactPathsMissingFromTop.length === 0
          ? 'ok'
          : rcFailureArtifactPathsMissingFromTop.join(','),
    },
    {
      id: 'release_candidate_rehearsal_chaos_artifact_paths_array_contract_v8',
      ok: Array.isArray(rcChaos?.artifact_paths) && rcChaosArtifactPaths.length > 0,
      detail: `count=${rcChaosArtifactPaths.length}`,
    },
    {
      id: 'release_candidate_rehearsal_chaos_artifact_paths_token_unique_contract_v8',
      ok: rcChaosArtifactPathsInvalid.length === 0 && rcChaosArtifactPathsDuplicate.length === 0,
      detail:
        rcChaosArtifactPathsInvalid.length === 0 && rcChaosArtifactPathsDuplicate.length === 0
          ? 'ok'
          : `invalid=${rcChaosArtifactPathsInvalid.join(',') || 'none'};duplicate=${Array.from(new Set(rcChaosArtifactPathsDuplicate)).join(',') || 'none'}`,
    },
    {
      id: 'release_candidate_rehearsal_replay_artifact_paths_array_contract_v8',
      ok: Array.isArray(rcReplay?.artifact_paths) && rcReplayArtifactPaths.length > 0,
      detail: `count=${rcReplayArtifactPaths.length}`,
    },
    {
      id: 'release_candidate_rehearsal_replay_artifact_paths_token_unique_contract_v8',
      ok: rcReplayArtifactPathsInvalid.length === 0 && rcReplayArtifactPathsDuplicate.length === 0,
      detail:
        rcReplayArtifactPathsInvalid.length === 0 && rcReplayArtifactPathsDuplicate.length === 0
          ? 'ok'
          : `invalid=${rcReplayArtifactPathsInvalid.join(',') || 'none'};duplicate=${Array.from(new Set(rcReplayArtifactPathsDuplicate)).join(',') || 'none'}`,
    },
    {
      id: 'release_candidate_rehearsal_layer2_parity_artifact_paths_subset_top_contract_v8',
      ok: rcLayer2ParityArtifactPathsMissingFromTop.length === 0,
      detail:
        rcLayer2ParityArtifactPathsMissingFromTop.length === 0
          ? 'ok'
          : rcLayer2ParityArtifactPathsMissingFromTop.join(','),
    },
    {
      id: 'release_candidate_rehearsal_layer2_receipt_replay_artifact_paths_subset_top_contract_v8',
      ok: rcLayer2ReceiptReplayArtifactPathsMissingFromTop.length === 0,
      detail:
        rcLayer2ReceiptReplayArtifactPathsMissingFromTop.length === 0
          ? 'ok'
          : rcLayer2ReceiptReplayArtifactPathsMissingFromTop.join(','),
    },
    {
      id: 'release_candidate_rehearsal_trusted_core_artifact_paths_subset_top_contract_v8',
      ok: rcTrustedCoreArtifactPathsMissingFromTop.length === 0,
      detail:
        rcTrustedCoreArtifactPathsMissingFromTop.length === 0
          ? 'ok'
          : rcTrustedCoreArtifactPathsMissingFromTop.join(','),
    },
    {
      id: 'release_candidate_rehearsal_proof_pack_artifact_paths_subset_top_contract_v8',
      ok: rcProofPackArtifactPathsMissingFromTop.length === 0,
      detail:
        rcProofPackArtifactPathsMissingFromTop.length === 0
          ? 'ok'
          : rcProofPackArtifactPathsMissingFromTop.join(','),
    },
    {
      id: 'release_candidate_rehearsal_chaos_artifact_paths_subset_top_contract_v8',
      ok: rcChaosArtifactPathsMissingFromTop.length === 0,
      detail:
        rcChaosArtifactPathsMissingFromTop.length === 0
          ? 'ok'
          : rcChaosArtifactPathsMissingFromTop.join(','),
    },
    {
      id: 'release_candidate_rehearsal_replay_artifact_paths_subset_top_contract_v8',
      ok: rcReplayArtifactPathsMissingFromTop.length === 0,
      detail:
        rcReplayArtifactPathsMissingFromTop.length === 0
          ? 'ok'
          : rcReplayArtifactPathsMissingFromTop.join(','),
    },
    {
      id: 'release_candidate_rehearsal_chaos_replay_expected_artifact_contract_v8',
      ok: rcChaosReplayExpectedArtifactsPresent,
      detail: `chaos_expected=${rcChaosExpectedArtifact || 'missing'};replay_expected=${rcReplayExpectedArtifact || 'missing'}`,
    },
    {
      id: 'release_candidate_rehearsal_step_gate_registry_contract_v2',
      ok: rcStepGateIdsMissingInRegistry.length === 0,
      detail:
        rcStepGateIdsMissingInRegistry.length === 0
          ? 'ok'
          : rcStepGateIdsMissingInRegistry.join(','),
    },
    {
      id: 'release_candidate_rehearsal_step_count_contract_v2',
      ok: rcSteps.length === rcStepCountExpected,
      detail: `actual=${rcSteps.length};expected=${rcStepCountExpected}`,
    },
    {
      id: 'release_candidate_rehearsal_present',
      ok: fs.existsSync(rcPath),
      detail: rcPath ? path.relative(root, rcPath) : 'missing',
    },
    {
      id: 'release_candidate_rehearsal_strict',
      ok: artifactStrict(rcPayload),
      detail: `strict=${String(rcPayload?.strict === true || rcPayload?.inputs?.strict === true)}`,
    },
    {
      id: 'release_candidate_rehearsal_candidate_ready',
      ok: rcPayload?.summary?.candidate_ready === true && rcPayload?.summary?.required_steps_satisfied === true,
      detail: `candidate_ready=${String(rcPayload?.summary?.candidate_ready === true)};required_steps=${String(rcPayload?.summary?.required_steps_satisfied === true)}`,
    },
  ];

  const requiredGateArtifactPathTokenInvalid: string[] = [];
  const requiredGateStepMissing: string[] = [];
  const requiredGateStepOkBooleanInvalid: string[] = [];
  const requiredGatePayloadMissing: string[] = [];
  const requiredGatePayloadNonObject: string[] = [];
  const requiredGatePayloadTypeMissing: string[] = [];
  const requiredGatePayloadGeneratedAtInvalid: string[] = [];
  const requiredGatePayloadOkBooleanInvalid: string[] = [];

  for (const [gateId, relPath] of Object.entries(requiredGateArtifacts)) {
    const artifactPath = resolveMaybe(root, String(relPath || ''));
    const payload = readJsonMaybe(artifactPath);
    const step = rcStepMap.get(gateId);

    if (!isCanonicalRelativePathToken(relPath, '', '.json')) {
      requiredGateArtifactPathTokenInvalid.push(`${gateId}:${relPath}`);
    }
    if (gateId !== 'ops:release:rc-rehearsal' && !step) {
      requiredGateStepMissing.push(gateId);
    }
    if (step && typeof step?.ok !== 'boolean') {
      requiredGateStepOkBooleanInvalid.push(gateId);
    }
    if (payload === null || payload === undefined) {
      requiredGatePayloadMissing.push(gateId);
    } else if (!isObjectRecord(payload)) {
      requiredGatePayloadNonObject.push(gateId);
    } else {
      const payloadType = String((payload as any)?.type || '').trim();
      const payloadGeneratedAt = String((payload as any)?.generated_at || '').trim();
      if (!payloadType) {
        requiredGatePayloadTypeMissing.push(gateId);
      }
      if (!payloadGeneratedAt || Number.isNaN(Date.parse(payloadGeneratedAt))) {
        requiredGatePayloadGeneratedAtInvalid.push(gateId);
      }
      if (typeof (payload as any)?.ok !== 'boolean') {
        requiredGatePayloadOkBooleanInvalid.push(gateId);
      }
    }

    checks.push({
      id: `release_gate_step:${gateId}`,
      ok: gateId === 'ops:release:rc-rehearsal' ? rcPayload?.ok === true : step?.ok === true,
      detail:
        gateId === 'ops:release:rc-rehearsal'
          ? `present=${String(rcPayload?.ok === true)}`
          : `present=${String(Boolean(step))};ok=${String(step?.ok === true)}`,
    });
    checks.push({
      id: `release_gate_artifact:${gateId}`,
      ok: fs.existsSync(artifactPath),
      detail: artifactPath ? path.relative(root, artifactPath) : 'missing',
    });
    checks.push({
      id: `release_gate_health:${gateId}`,
      ok: artifactOk(gateId, payload),
      detail: `artifact_ok=${String(artifactOk(gateId, payload))}`,
    });
    if (gateId === 'release_policy_gate' || gateId === 'ops:release:scorecard:gate' || gateId === 'ops:production-closure:gate') {
      checks.push({
        id: `release_gate_strict_artifact:${gateId}`,
        ok: artifactStrict(payload),
        detail: `strict=${String(artifactStrict(payload))}`,
      });
    }
    if (gateId === 'ops:runtime-proof:verify') {
      const profiles = Array.isArray(payload?.profile_runs) ? payload.profile_runs : [];
      const profileCount = Number(payload?.summary?.profile_count || 0);
      const proofTrack = String(payload?.summary?.proof_track || '').trim();
      const empiricalOk = profiles.every((row: any) => row?.empirical_sample_points_ok === true);
      const runtimeProofProfileIds = profiles.map((row: any) => String(row?.profile || '').trim());
      const runtimeProofProfileIdInvalid = runtimeProofProfileIds.filter(
        (profile) => !/^[a-z0-9-]+$/.test(profile),
      );
      const runtimeProofProfileIdDuplicate = runtimeProofProfileIds.filter(
        (profile, index, arr) => arr.indexOf(profile) !== index,
      );
      const runtimeProofRequiredProfiles = ['rich', 'pure', 'tiny-max'];
      const runtimeProofMissingRequiredProfiles = runtimeProofRequiredProfiles.filter(
        (profile) => !runtimeProofProfileIds.includes(profile),
      );
      const runtimeProofOrderMatchesCanonical =
        runtimeProofProfileIds.join(',') === runtimeProofRequiredProfiles.join(',');
      const runtimeProofEmpiricalSamplePointFlagInvalid = profiles
        .filter((row: any) => typeof row?.empirical_sample_points_ok !== 'boolean')
        .map((row: any) => String(row?.profile || 'unknown'));
      checks.push({
        id: 'runtime_proof_dual_track_mandatory',
        ok: proofTrack === 'dual',
        detail: `proof_track=${proofTrack || 'unknown'}`,
      });
      checks.push({
        id: 'runtime_proof_profile_coverage_all',
        ok: profileCount >= 3,
        detail: `profile_count=${profileCount}`,
      });
      checks.push({
        id: 'runtime_proof_empirical_nonzero_all_profiles',
        ok: empiricalOk,
        detail: `empirical_nonzero_all_profiles=${String(empiricalOk)}`,
      });
      checks.push({
        id: 'runtime_proof_profile_rows_match_summary_count_contract_v9',
        ok: profiles.length === profileCount,
        detail: `rows=${profiles.length};summary_count=${profileCount}`,
      });
      checks.push({
        id: 'runtime_proof_profile_ids_token_contract_v9',
        ok: runtimeProofProfileIdInvalid.length === 0,
        detail:
          runtimeProofProfileIdInvalid.length === 0 ? 'ok' : runtimeProofProfileIdInvalid.join(','),
      });
      checks.push({
        id: 'runtime_proof_profile_ids_unique_contract_v9',
        ok: runtimeProofProfileIdDuplicate.length === 0,
        detail:
          runtimeProofProfileIdDuplicate.length === 0
            ? 'ok'
            : Array.from(new Set(runtimeProofProfileIdDuplicate)).join(','),
      });
      checks.push({
        id: 'runtime_proof_profile_required_set_contract_v9',
        ok: runtimeProofMissingRequiredProfiles.length === 0,
        detail:
          runtimeProofMissingRequiredProfiles.length === 0
            ? 'ok'
            : runtimeProofMissingRequiredProfiles.join(','),
      });
      checks.push({
        id: 'runtime_proof_profile_order_contract_v9',
        ok: runtimeProofOrderMatchesCanonical,
        detail: runtimeProofProfileIds.join(','),
      });
      checks.push({
        id: 'runtime_proof_profile_empirical_sample_points_flag_contract_v9',
        ok: runtimeProofEmpiricalSamplePointFlagInvalid.length === 0,
        detail:
          runtimeProofEmpiricalSamplePointFlagInvalid.length === 0
            ? 'ok'
            : runtimeProofEmpiricalSamplePointFlagInvalid.join(','),
      });
    }
    if (gateId === 'ops:release:proof-pack') {
      const requiredMissing = Number(
        payload?.summary?.required_missing ?? (Array.isArray(payload?.required_missing) ? payload.required_missing.length : 0),
      );
      const categoryThresholdFailures = Number(
        payload?.summary?.category_threshold_failure_count ??
          (Array.isArray(payload?.category_threshold_failures) ? payload.category_threshold_failures.length : 0),
      );
      const proofPackSummaryIsObject = isObjectRecord(payload?.summary);
      const proofPackArtifactCount = Number(payload?.summary?.artifact_count ?? -1);
      const proofPackPass = payload?.summary?.pass;
      const proofPackScalarsValid =
        Number.isInteger(requiredMissing) &&
        requiredMissing >= 0 &&
        Number.isInteger(categoryThresholdFailures) &&
        categoryThresholdFailures >= 0 &&
        Number.isInteger(proofPackArtifactCount) &&
        proofPackArtifactCount >= 0;
      const proofPackPassConsistency =
        typeof proofPackPass === 'boolean'
          ? proofPackPass === (requiredMissing === 0 && categoryThresholdFailures === 0)
          : false;
      const proofPackExistingArtifactsRaw = Array.isArray(payload?.existing_artifacts)
        ? payload.existing_artifacts
        : Array.isArray(payload?.artifacts)
          ? payload.artifacts
          : [];
      const proofPackExistingArtifactPaths = proofPackExistingArtifactsRaw.map((row: any) => {
        if (typeof row === 'string') {
          return row.trim();
        }
        if (isObjectRecord(row)) {
          return String(row.destination_path ?? row.path ?? row.source_path ?? '').trim();
        }
        return '';
      });
      const proofPackExistingArtifactPathInvalid = proofPackExistingArtifactPaths.filter(
        (artifactPath) => !artifactPath,
      );
      const proofPackExistingArtifactPathDuplicate = proofPackExistingArtifactPaths.filter(
        (artifactPath, index, arr) => artifactPath && arr.indexOf(artifactPath) !== index,
      );
      checks.push({
        id: 'release_proof_pack_required_missing_zero',
        ok: requiredMissing === 0,
        detail: `required_missing=${requiredMissing}`,
      });
      checks.push({
        id: 'release_proof_pack_category_thresholds_met',
        ok: categoryThresholdFailures === 0,
        detail: `category_threshold_failures=${categoryThresholdFailures}`,
      });
      checks.push({
        id: 'release_proof_pack_summary_object_contract_v9',
        ok: proofPackSummaryIsObject,
        detail: `summary_object=${String(proofPackSummaryIsObject)}`,
      });
      checks.push({
        id: 'release_proof_pack_summary_scalar_contract_v9',
        ok: proofPackScalarsValid,
        detail: `required_missing=${requiredMissing};category_threshold_failures=${categoryThresholdFailures};artifact_count=${proofPackArtifactCount}`,
      });
      checks.push({
        id: 'release_proof_pack_pass_consistency_contract_v9',
        ok: proofPackPassConsistency,
        detail: `pass=${String(proofPackPass === true)};required_missing=${requiredMissing};category_threshold_failures=${categoryThresholdFailures}`,
      });
      checks.push({
        id: 'release_proof_pack_existing_artifacts_array_contract_v9',
        ok: proofPackExistingArtifactsRaw.length > 0,
        detail: `count=${proofPackExistingArtifactsRaw.length}`,
      });
      checks.push({
        id: 'release_proof_pack_existing_artifact_path_contract_v9',
        ok: proofPackExistingArtifactPathInvalid.length === 0,
        detail:
          proofPackExistingArtifactPathInvalid.length === 0
            ? 'ok'
            : `missing_path_entries=${proofPackExistingArtifactPathInvalid.length}`,
      });
      checks.push({
        id: 'release_proof_pack_existing_artifact_unique_contract_v9',
        ok: proofPackExistingArtifactPathDuplicate.length === 0,
        detail:
          proofPackExistingArtifactPathDuplicate.length === 0
            ? 'ok'
            : Array.from(new Set(proofPackExistingArtifactPathDuplicate)).join(','),
      });
    }
  }

  checks.push({
    id: 'release_verdict_required_gate_artifact_path_token_contract_v9',
    ok: requiredGateArtifactPathTokenInvalid.length === 0,
    detail:
      requiredGateArtifactPathTokenInvalid.length === 0
        ? 'ok'
        : requiredGateArtifactPathTokenInvalid.join(','),
  });
  checks.push({
    id: 'release_verdict_required_gate_step_presence_contract_v9',
    ok: requiredGateStepMissing.length === 0,
    detail: requiredGateStepMissing.length === 0 ? 'ok' : requiredGateStepMissing.join(','),
  });
  checks.push({
    id: 'release_verdict_required_gate_step_ok_boolean_contract_v9',
    ok: requiredGateStepOkBooleanInvalid.length === 0,
    detail:
      requiredGateStepOkBooleanInvalid.length === 0
        ? 'ok'
        : requiredGateStepOkBooleanInvalid.join(','),
  });
  checks.push({
    id: 'release_verdict_required_gate_payload_presence_contract_v9',
    ok: requiredGatePayloadMissing.length === 0,
    detail: requiredGatePayloadMissing.length === 0 ? 'ok' : requiredGatePayloadMissing.join(','),
  });
  checks.push({
    id: 'release_verdict_required_gate_payload_object_contract_v9',
    ok: requiredGatePayloadNonObject.length === 0,
    detail:
      requiredGatePayloadNonObject.length === 0 ? 'ok' : requiredGatePayloadNonObject.join(','),
  });
  checks.push({
    id: 'release_verdict_required_gate_payload_type_contract_v9',
    ok: requiredGatePayloadTypeMissing.length === 0,
    detail:
      requiredGatePayloadTypeMissing.length === 0
        ? 'ok'
        : requiredGatePayloadTypeMissing.join(','),
  });
  checks.push({
    id: 'release_verdict_required_gate_payload_generated_at_contract_v9',
    ok: requiredGatePayloadGeneratedAtInvalid.length === 0,
    detail:
      requiredGatePayloadGeneratedAtInvalid.length === 0
        ? 'ok'
        : requiredGatePayloadGeneratedAtInvalid.join(','),
  });
  checks.push({
    id: 'release_verdict_required_gate_payload_ok_boolean_contract_v9',
    ok: requiredGatePayloadOkBooleanInvalid.length === 0,
    detail:
      requiredGatePayloadOkBooleanInvalid.length === 0
        ? 'ok'
        : requiredGatePayloadOkBooleanInvalid.join(','),
  });

  const artifact_hashes = checksumArtifactPaths.map((relPath: string) => {
    const artifactPath = resolveMaybe(root, relPath);
    const exists = fs.existsSync(artifactPath);
    return {
      path: relPath,
      exists,
      sha256: exists ? fileDigest(artifactPath) : '',
    };
  });
  const verdict_checksum = crypto
    .createHash('sha256')
    .update(
      artifact_hashes
        .map((row) => `${row.path}:${row.exists ? row.sha256 : 'missing'}`)
        .join('\n'),
    )
    .digest('hex');
  const artifactHashPaths = artifact_hashes.map((row) => cleanText(row.path || '', 500));
  const artifactHashPathTokensInvalid = artifactHashPaths.filter(
    (relPath) => !isCanonicalRelativePathToken(relPath, '', '.json'),
  );
  const artifactHashShaInvalid = artifact_hashes
    .filter((row) => row.exists && !/^[a-f0-9]{64}$/.test(cleanText(row.sha256 || '', 80)))
    .map((row) => cleanText(row.path || '', 500));
  const artifactHashMissingCount = artifact_hashes.filter((row) => !row.exists).length;
  const checkIds = checks.map((row) => cleanText(row.id || '', 220)).filter(Boolean);
  const checkIdsDuplicate = checkIds.filter((id, idx, arr) => id && arr.indexOf(id) !== idx);
  const failedIdsPrecomputed = checks
    .filter((row) => !row.ok)
    .map((row) => cleanText(row.id || '', 220))
    .filter(Boolean);
  const failedIdsDuplicate = failedIdsPrecomputed.filter((id, idx, arr) => id && arr.indexOf(id) !== idx);
  const failedIdsUnknown = failedIdsPrecomputed.filter((id) => !checkIds.includes(id));
  checks.push(
    {
      id: 'release_verdict_artifact_hash_count_matches_checksum_contract_v2',
      ok: artifact_hashes.length === checksumPathsTrimmed.length,
      detail: `hashes=${artifact_hashes.length};checksums=${checksumPathsTrimmed.length}`,
    },
    {
      id: 'release_verdict_artifact_hash_path_order_contract_v2',
      ok:
        artifactHashPaths.length === checksumPathsTrimmed.length &&
        artifactHashPaths.every((relPath, idx) => relPath === checksumPathsTrimmed[idx]),
      detail: `order_match=${String(
        artifactHashPaths.length === checksumPathsTrimmed.length &&
          artifactHashPaths.every((relPath, idx) => relPath === checksumPathsTrimmed[idx]),
      )}`,
    },
    {
      id: 'release_verdict_artifact_hash_path_token_contract_v2',
      ok: artifactHashPathTokensInvalid.length === 0,
      detail:
        artifactHashPathTokensInvalid.length === 0
          ? 'ok'
          : artifactHashPathTokensInvalid.join(','),
    },
    {
      id: 'release_verdict_artifact_hash_sha256_shape_contract_v2',
      ok: artifactHashShaInvalid.length === 0,
      detail: artifactHashShaInvalid.length === 0 ? 'ok' : artifactHashShaInvalid.join(','),
    },
    {
      id: 'release_verdict_checksum_shape_contract_v2',
      ok: /^[a-f0-9]{64}$/.test(cleanText(verdict_checksum || '', 80)),
      detail: verdict_checksum || 'missing',
    },
    {
      id: 'release_verdict_artifact_hash_all_present_contract_v2',
      ok: artifactHashMissingCount === 0,
      detail: `missing=${artifactHashMissingCount}`,
    },
    {
      id: 'release_verdict_check_ids_unique_contract_v2',
      ok: checkIdsDuplicate.length === 0,
      detail:
        checkIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(checkIdsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_failed_ids_unique_subset_contract_v2',
      ok: failedIdsDuplicate.length === 0 && failedIdsUnknown.length === 0,
      detail:
        failedIdsDuplicate.length === 0 && failedIdsUnknown.length === 0
          ? 'ok'
          : `duplicate=${Array.from(new Set(failedIdsDuplicate)).join(',') || 'none'};unknown=${failedIdsUnknown.join(',') || 'none'}`,
    },
  );
  const artifactHashRowsObjectInvalidCount = artifact_hashes.filter((row) => !isObjectRecord(row)).length;
  const artifactHashExistsBooleanInvalid = artifact_hashes
    .filter((row: any) => typeof row?.exists !== 'boolean')
    .map((row) => cleanText(row?.path || '', 500));
  const artifactHashMissingShaNotEmpty = artifact_hashes
    .filter((row) => !row.exists && cleanText(row.sha256 || '', 80).length > 0)
    .map((row) => cleanText(row.path || '', 500));
  const artifactHashExistingShaMissing = artifact_hashes
    .filter((row) => row.exists && cleanText(row.sha256 || '', 80).length === 0)
    .map((row) => cleanText(row.path || '', 500));
  const artifactHashPathDuplicate = artifactHashPaths.filter(
    (relPath, index, arr) => relPath && arr.indexOf(relPath) !== index,
  );
  const artifactHashPathsMissingInChecksums = artifactHashPaths.filter(
    (relPath) => !checksumPathsTrimmed.includes(relPath),
  );
  const checksumPathsMissingInArtifactHashes = checksumPathsTrimmed.filter(
    (relPath) => !artifactHashPaths.includes(relPath),
  );
  const requiredGateArtifactPathsMissingInChecksumPaths = requiredGateArtifactPaths.filter(
    (relPath) => !checksumPathsTrimmed.includes(relPath),
  );
  const requiredGateArtifactPathsMissingInArtifactHashes = requiredGateArtifactPaths.filter(
    (relPath) => !artifactHashPaths.includes(relPath),
  );
  const checkIdTokenRegex = /^[a-z0-9_:-]+$/;
  const checkRowsObjectInvalidCount = checks.filter((row) => !isObjectRecord(row)).length;
  const checkIdTokenInvalid = checks
    .map((row) => cleanText((row as any)?.id || '', 220))
    .filter((id) => !id || !checkIdTokenRegex.test(id));
  const checkOkBooleanInvalid = checks
    .filter((row: any) => typeof row?.ok !== 'boolean')
    .map((row) => cleanText((row as any)?.id || '', 220));
  const checkDetailMissing = checks
    .filter((row: any) => typeof row?.detail !== 'string' || !row.detail.trim())
    .map((row) => cleanText((row as any)?.id || '', 220));
  const failedIdsFromChecksPostV2 = checks
    .filter((row) => !row.ok)
    .map((row) => cleanText(row.id || '', 220))
    .filter(Boolean);
  const failedIdsTokenInvalidPostV2 = failedIdsFromChecksPostV2.filter(
    (id) => !checkIdTokenRegex.test(id),
  );
  const failedIdsOrderMatchesChecksPostV2 =
    failedIdsFromChecksPostV2.join(',') ===
    checks
      .filter((row) => !row.ok)
      .map((row) => cleanText(row.id || '', 220))
      .join(',');
  const requiredGateStepRowsMissing = requiredGateIds.filter(
    (gateId) => !checks.some((row) => row.id === `release_gate_step:${gateId}`),
  );
  const requiredGateArtifactRowsMissing = requiredGateIds.filter(
    (gateId) => !checks.some((row) => row.id === `release_gate_artifact:${gateId}`),
  );
  const requiredGateHealthRowsMissing = requiredGateIds.filter(
    (gateId) => !checks.some((row) => row.id === `release_gate_health:${gateId}`),
  );
  const mandatoryBaselineCheckIds = [
    'release_verdict_required_gate_expected_set_contract_v2',
    'runtime_proof_dual_track_mandatory',
    'release_proof_pack_required_missing_zero',
    'release_candidate_rehearsal_candidate_ready',
    'release_verdict_checksum_shape_contract_v2',
  ];
  const mandatoryBaselineCheckIdsMissing = mandatoryBaselineCheckIds.filter(
    (id) => !checks.some((row) => row.id === id),
  );
  checks.push(
    {
      id: 'release_verdict_artifact_hash_rows_object_contract_v10',
      ok: artifactHashRowsObjectInvalidCount === 0,
      detail: `invalid_rows=${artifactHashRowsObjectInvalidCount}`,
    },
    {
      id: 'release_verdict_artifact_hash_exists_boolean_contract_v10',
      ok: artifactHashExistsBooleanInvalid.length === 0,
      detail:
        artifactHashExistsBooleanInvalid.length === 0
          ? 'ok'
          : artifactHashExistsBooleanInvalid.join(','),
    },
    {
      id: 'release_verdict_artifact_hash_missing_sha_empty_contract_v10',
      ok: artifactHashMissingShaNotEmpty.length === 0,
      detail:
        artifactHashMissingShaNotEmpty.length === 0
          ? 'ok'
          : artifactHashMissingShaNotEmpty.join(','),
    },
    {
      id: 'release_verdict_artifact_hash_existing_sha_present_contract_v10',
      ok: artifactHashExistingShaMissing.length === 0,
      detail:
        artifactHashExistingShaMissing.length === 0
          ? 'ok'
          : artifactHashExistingShaMissing.join(','),
    },
    {
      id: 'release_verdict_artifact_hash_paths_unique_contract_v10',
      ok: artifactHashPathDuplicate.length === 0,
      detail:
        artifactHashPathDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(artifactHashPathDuplicate)).join(','),
    },
    {
      id: 'release_verdict_artifact_hash_paths_set_matches_checksums_contract_v10',
      ok:
        artifactHashPathsMissingInChecksums.length === 0 &&
        checksumPathsMissingInArtifactHashes.length === 0,
      detail:
        artifactHashPathsMissingInChecksums.length === 0 &&
        checksumPathsMissingInArtifactHashes.length === 0
          ? 'ok'
          : `hash_only=${artifactHashPathsMissingInChecksums.join(',') || 'none'};checksum_only=${checksumPathsMissingInArtifactHashes.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_required_gate_artifact_paths_in_checksum_contract_v10',
      ok: requiredGateArtifactPathsMissingInChecksumPaths.length === 0,
      detail:
        requiredGateArtifactPathsMissingInChecksumPaths.length === 0
          ? 'ok'
          : requiredGateArtifactPathsMissingInChecksumPaths.join(','),
    },
    {
      id: 'release_verdict_required_gate_artifact_paths_in_hash_rows_contract_v10',
      ok: requiredGateArtifactPathsMissingInArtifactHashes.length === 0,
      detail:
        requiredGateArtifactPathsMissingInArtifactHashes.length === 0
          ? 'ok'
          : requiredGateArtifactPathsMissingInArtifactHashes.join(','),
    },
    {
      id: 'release_verdict_checks_array_nonempty_contract_v10',
      ok: Array.isArray(checks) && checks.length > 0,
      detail: `count=${checks.length}`,
    },
    {
      id: 'release_verdict_check_rows_object_contract_v10',
      ok: checkRowsObjectInvalidCount === 0,
      detail: `invalid_rows=${checkRowsObjectInvalidCount}`,
    },
    {
      id: 'release_verdict_check_id_token_contract_v10',
      ok: checkIdTokenInvalid.length === 0,
      detail: checkIdTokenInvalid.length === 0 ? 'ok' : checkIdTokenInvalid.join(','),
    },
    {
      id: 'release_verdict_check_ok_boolean_contract_v10',
      ok: checkOkBooleanInvalid.length === 0,
      detail: checkOkBooleanInvalid.length === 0 ? 'ok' : checkOkBooleanInvalid.join(','),
    },
    {
      id: 'release_verdict_check_detail_nonempty_contract_v10',
      ok: checkDetailMissing.length === 0,
      detail: checkDetailMissing.length === 0 ? 'ok' : checkDetailMissing.join(','),
    },
    {
      id: 'release_verdict_failed_ids_match_failed_checks_contract_v10',
      ok: failedIdsFromChecksPostV2.join(',') === failedIdsPrecomputed.join(','),
      detail: `post_v2=${failedIdsFromChecksPostV2.length};precomputed=${failedIdsPrecomputed.length}`,
    },
    {
      id: 'release_verdict_failed_ids_order_matches_failed_checks_contract_v10',
      ok: failedIdsOrderMatchesChecksPostV2,
      detail: `order_match=${String(failedIdsOrderMatchesChecksPostV2)}`,
    },
    {
      id: 'release_verdict_failed_ids_token_contract_v10',
      ok: failedIdsTokenInvalidPostV2.length === 0,
      detail:
        failedIdsTokenInvalidPostV2.length === 0
          ? 'ok'
          : failedIdsTokenInvalidPostV2.join(','),
    },
    {
      id: 'release_verdict_failed_ids_nonempty_on_failure_contract_v10',
      ok: failedIdsPrecomputed.every((id) => failedIdsFromChecksPostV2.includes(id)),
      detail: `precomputed=${failedIdsPrecomputed.length};post_v2=${failedIdsFromChecksPostV2.length}`,
    },
    {
      id: 'release_verdict_failed_ids_empty_on_success_contract_v10',
      ok: failedIdsFromChecksPostV2.every((id) => failedIdsPrecomputed.includes(id)),
      detail: `post_v2=${failedIdsFromChecksPostV2.length};precomputed=${failedIdsPrecomputed.length}`,
    },
    {
      id: 'release_verdict_required_gate_step_artifact_health_rows_present_contract_v10',
      ok:
        requiredGateStepRowsMissing.length === 0 &&
        requiredGateArtifactRowsMissing.length === 0 &&
        requiredGateHealthRowsMissing.length === 0,
      detail:
        requiredGateStepRowsMissing.length === 0 &&
        requiredGateArtifactRowsMissing.length === 0 &&
        requiredGateHealthRowsMissing.length === 0
          ? 'ok'
          : `step_missing=${requiredGateStepRowsMissing.join(',') || 'none'};artifact_missing=${requiredGateArtifactRowsMissing.join(',') || 'none'};health_missing=${requiredGateHealthRowsMissing.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_mandatory_baseline_checks_present_contract_v10',
      ok: mandatoryBaselineCheckIdsMissing.length === 0,
      detail:
        mandatoryBaselineCheckIdsMissing.length === 0
          ? 'ok'
          : mandatoryBaselineCheckIdsMissing.join(','),
    },
  );
  const requiredGateStepCheckIdsExpected = requiredGateIds.map(
    (gateId) => `release_gate_step:${gateId}`,
  );
  const requiredGateArtifactCheckIdsExpected = requiredGateIds.map(
    (gateId) => `release_gate_artifact:${gateId}`,
  );
  const requiredGateHealthCheckIdsExpected = requiredGateIds.map(
    (gateId) => `release_gate_health:${gateId}`,
  );
  const requiredGateStepRows = checks.filter((row) => row.id.startsWith('release_gate_step:'));
  const requiredGateArtifactRows = checks.filter((row) =>
    row.id.startsWith('release_gate_artifact:'),
  );
  const requiredGateHealthRows = checks.filter((row) => row.id.startsWith('release_gate_health:'));
  const requiredGateStepRowsIds = requiredGateStepRows.map((row) => cleanText(row.id || '', 260));
  const requiredGateArtifactRowsIds = requiredGateArtifactRows.map((row) =>
    cleanText(row.id || '', 260),
  );
  const requiredGateHealthRowsIds = requiredGateHealthRows.map((row) =>
    cleanText(row.id || '', 260),
  );
  const requiredGateStepRowsDuplicate = requiredGateStepRowsIds.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const requiredGateArtifactRowsDuplicate = requiredGateArtifactRowsIds.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const requiredGateHealthRowsDuplicate = requiredGateHealthRowsIds.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const requiredGateStepRowsMissingExpected = requiredGateStepCheckIdsExpected.filter(
    (id) => !requiredGateStepRowsIds.includes(id),
  );
  const requiredGateStepRowsUnexpected = requiredGateStepRowsIds.filter(
    (id) => !requiredGateStepCheckIdsExpected.includes(id),
  );
  const requiredGateArtifactRowsMissingExpected = requiredGateArtifactCheckIdsExpected.filter(
    (id) => !requiredGateArtifactRowsIds.includes(id),
  );
  const requiredGateArtifactRowsUnexpected = requiredGateArtifactRowsIds.filter(
    (id) => !requiredGateArtifactCheckIdsExpected.includes(id),
  );
  const requiredGateHealthRowsMissingExpected = requiredGateHealthCheckIdsExpected.filter(
    (id) => !requiredGateHealthRowsIds.includes(id),
  );
  const requiredGateHealthRowsUnexpected = requiredGateHealthRowsIds.filter(
    (id) => !requiredGateHealthCheckIdsExpected.includes(id),
  );
  const requiredGateStepRowsDetailInvalid = requiredGateStepRows
    .filter((row: any) => !cleanText(row?.detail || '', 1000).includes('present='))
    .map((row) => cleanText(row.id || '', 260));
  const requiredGateArtifactRowsDetailInvalid = requiredGateArtifactRows
    .filter((row: any) => cleanText(row?.detail || '', 1000).length === 0)
    .map((row) => cleanText(row.id || '', 260));
  const requiredGateHealthRowsDetailInvalid = requiredGateHealthRows
    .filter((row: any) => !cleanText(row?.detail || '', 1000).includes('artifact_ok='))
    .map((row) => cleanText(row.id || '', 260));
  const strictGateCheckIdsExpected = [
    'release_gate_strict_artifact:release_policy_gate',
    'release_gate_strict_artifact:ops:release:scorecard:gate',
    'release_gate_strict_artifact:ops:production-closure:gate',
  ];
  const strictGateRows = checks.filter((row) => row.id.startsWith('release_gate_strict_artifact:'));
  const strictGateRowsIds = strictGateRows.map((row) => cleanText(row.id || '', 260));
  const strictGateRowsMissingExpected = strictGateCheckIdsExpected.filter(
    (id) => !strictGateRowsIds.includes(id),
  );
  const strictGateRowsUnexpected = strictGateRowsIds.filter(
    (id) => !strictGateCheckIdsExpected.includes(id),
  );
  const strictGateRowsDetailInvalid = strictGateRows
    .filter((row: any) => !cleanText(row?.detail || '', 1000).includes('strict='))
    .map((row) => cleanText(row.id || '', 260));
  const runtimeProofContractIdsV9 = [
    'runtime_proof_dual_track_mandatory',
    'runtime_proof_profile_coverage_all',
    'runtime_proof_empirical_nonzero_all_profiles',
    'runtime_proof_profile_rows_match_summary_count_contract_v9',
    'runtime_proof_profile_ids_token_contract_v9',
    'runtime_proof_profile_ids_unique_contract_v9',
    'runtime_proof_profile_required_set_contract_v9',
    'runtime_proof_profile_order_contract_v9',
    'runtime_proof_profile_empirical_sample_points_flag_contract_v9',
  ];
  const proofPackContractIdsV9 = [
    'release_proof_pack_required_missing_zero',
    'release_proof_pack_category_thresholds_met',
    'release_proof_pack_summary_object_contract_v9',
    'release_proof_pack_summary_scalar_contract_v9',
    'release_proof_pack_pass_consistency_contract_v9',
    'release_proof_pack_existing_artifacts_array_contract_v9',
    'release_proof_pack_existing_artifact_path_contract_v9',
    'release_proof_pack_existing_artifact_unique_contract_v9',
  ];
  const requiredGatePayloadContractIdsV9 = [
    'release_verdict_required_gate_artifact_path_token_contract_v9',
    'release_verdict_required_gate_step_presence_contract_v9',
    'release_verdict_required_gate_step_ok_boolean_contract_v9',
    'release_verdict_required_gate_payload_presence_contract_v9',
    'release_verdict_required_gate_payload_object_contract_v9',
    'release_verdict_required_gate_payload_type_contract_v9',
    'release_verdict_required_gate_payload_generated_at_contract_v9',
    'release_verdict_required_gate_payload_ok_boolean_contract_v9',
  ];
  const v10ContractIds = [
    'release_verdict_artifact_hash_rows_object_contract_v10',
    'release_verdict_artifact_hash_exists_boolean_contract_v10',
    'release_verdict_artifact_hash_missing_sha_empty_contract_v10',
    'release_verdict_artifact_hash_existing_sha_present_contract_v10',
    'release_verdict_artifact_hash_paths_unique_contract_v10',
    'release_verdict_artifact_hash_paths_set_matches_checksums_contract_v10',
    'release_verdict_required_gate_artifact_paths_in_checksum_contract_v10',
    'release_verdict_required_gate_artifact_paths_in_hash_rows_contract_v10',
    'release_verdict_checks_array_nonempty_contract_v10',
    'release_verdict_check_rows_object_contract_v10',
    'release_verdict_check_id_token_contract_v10',
    'release_verdict_check_ok_boolean_contract_v10',
    'release_verdict_check_detail_nonempty_contract_v10',
    'release_verdict_failed_ids_match_failed_checks_contract_v10',
    'release_verdict_failed_ids_order_matches_failed_checks_contract_v10',
    'release_verdict_failed_ids_token_contract_v10',
    'release_verdict_failed_ids_nonempty_on_failure_contract_v10',
    'release_verdict_failed_ids_empty_on_success_contract_v10',
    'release_verdict_required_gate_step_artifact_health_rows_present_contract_v10',
    'release_verdict_mandatory_baseline_checks_present_contract_v10',
  ];
  const postV10CheckIds = checks.map((row) => cleanText(row.id || '', 260)).filter(Boolean);
  const postV10CheckIdsDuplicate = postV10CheckIds.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const runtimeProofContractIdsMissingV9 = runtimeProofContractIdsV9.filter(
    (id) => !postV10CheckIds.includes(id),
  );
  const proofPackContractIdsMissingV9 = proofPackContractIdsV9.filter(
    (id) => !postV10CheckIds.includes(id),
  );
  const requiredGatePayloadContractIdsMissingV9 = requiredGatePayloadContractIdsV9.filter(
    (id) => !postV10CheckIds.includes(id),
  );
  const v10ContractIdsMissing = v10ContractIds.filter((id) => !postV10CheckIds.includes(id));
  checks.push(
    {
      id: 'release_verdict_required_gate_step_rows_count_contract_v11',
      ok: requiredGateStepRows.length === requiredGateIds.length,
      detail: `rows=${requiredGateStepRows.length};required=${requiredGateIds.length}`,
    },
    {
      id: 'release_verdict_required_gate_step_rows_unique_contract_v11',
      ok: requiredGateStepRowsDuplicate.length === 0,
      detail:
        requiredGateStepRowsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(requiredGateStepRowsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_required_gate_step_rows_set_contract_v11',
      ok:
        requiredGateStepRowsMissingExpected.length === 0 &&
        requiredGateStepRowsUnexpected.length === 0,
      detail:
        requiredGateStepRowsMissingExpected.length === 0 &&
        requiredGateStepRowsUnexpected.length === 0
          ? 'ok'
          : `missing=${requiredGateStepRowsMissingExpected.join(',') || 'none'};unexpected=${requiredGateStepRowsUnexpected.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_required_gate_step_rows_detail_contract_v11',
      ok: requiredGateStepRowsDetailInvalid.length === 0,
      detail:
        requiredGateStepRowsDetailInvalid.length === 0
          ? 'ok'
          : requiredGateStepRowsDetailInvalid.join(','),
    },
    {
      id: 'release_verdict_required_gate_artifact_rows_count_contract_v11',
      ok: requiredGateArtifactRows.length === requiredGateIds.length,
      detail: `rows=${requiredGateArtifactRows.length};required=${requiredGateIds.length}`,
    },
    {
      id: 'release_verdict_required_gate_artifact_rows_unique_contract_v11',
      ok: requiredGateArtifactRowsDuplicate.length === 0,
      detail:
        requiredGateArtifactRowsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(requiredGateArtifactRowsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_required_gate_artifact_rows_set_contract_v11',
      ok:
        requiredGateArtifactRowsMissingExpected.length === 0 &&
        requiredGateArtifactRowsUnexpected.length === 0,
      detail:
        requiredGateArtifactRowsMissingExpected.length === 0 &&
        requiredGateArtifactRowsUnexpected.length === 0
          ? 'ok'
          : `missing=${requiredGateArtifactRowsMissingExpected.join(',') || 'none'};unexpected=${requiredGateArtifactRowsUnexpected.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_required_gate_artifact_rows_detail_contract_v11',
      ok: requiredGateArtifactRowsDetailInvalid.length === 0,
      detail:
        requiredGateArtifactRowsDetailInvalid.length === 0
          ? 'ok'
          : requiredGateArtifactRowsDetailInvalid.join(','),
    },
    {
      id: 'release_verdict_required_gate_health_rows_count_contract_v11',
      ok: requiredGateHealthRows.length === requiredGateIds.length,
      detail: `rows=${requiredGateHealthRows.length};required=${requiredGateIds.length}`,
    },
    {
      id: 'release_verdict_required_gate_health_rows_unique_contract_v11',
      ok: requiredGateHealthRowsDuplicate.length === 0,
      detail:
        requiredGateHealthRowsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(requiredGateHealthRowsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_required_gate_health_rows_set_contract_v11',
      ok:
        requiredGateHealthRowsMissingExpected.length === 0 &&
        requiredGateHealthRowsUnexpected.length === 0,
      detail:
        requiredGateHealthRowsMissingExpected.length === 0 &&
        requiredGateHealthRowsUnexpected.length === 0
          ? 'ok'
          : `missing=${requiredGateHealthRowsMissingExpected.join(',') || 'none'};unexpected=${requiredGateHealthRowsUnexpected.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_required_gate_health_rows_detail_contract_v11',
      ok: requiredGateHealthRowsDetailInvalid.length === 0,
      detail:
        requiredGateHealthRowsDetailInvalid.length === 0
          ? 'ok'
          : requiredGateHealthRowsDetailInvalid.join(','),
    },
    {
      id: 'release_verdict_required_gate_strict_rows_count_contract_v11',
      ok: strictGateRows.length === strictGateCheckIdsExpected.length,
      detail: `rows=${strictGateRows.length};required=${strictGateCheckIdsExpected.length}`,
    },
    {
      id: 'release_verdict_required_gate_strict_rows_set_contract_v11',
      ok: strictGateRowsMissingExpected.length === 0 && strictGateRowsUnexpected.length === 0,
      detail:
        strictGateRowsMissingExpected.length === 0 && strictGateRowsUnexpected.length === 0
          ? 'ok'
          : `missing=${strictGateRowsMissingExpected.join(',') || 'none'};unexpected=${strictGateRowsUnexpected.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_required_gate_strict_rows_detail_contract_v11',
      ok: strictGateRowsDetailInvalid.length === 0,
      detail:
        strictGateRowsDetailInvalid.length === 0
          ? 'ok'
          : strictGateRowsDetailInvalid.join(','),
    },
    {
      id: 'release_verdict_runtime_proof_contract_rows_present_contract_v11',
      ok: runtimeProofContractIdsMissingV9.length === 0,
      detail:
        runtimeProofContractIdsMissingV9.length === 0
          ? 'ok'
          : runtimeProofContractIdsMissingV9.join(','),
    },
    {
      id: 'release_verdict_proof_pack_contract_rows_present_contract_v11',
      ok: proofPackContractIdsMissingV9.length === 0,
      detail:
        proofPackContractIdsMissingV9.length === 0
          ? 'ok'
          : proofPackContractIdsMissingV9.join(','),
    },
    {
      id: 'release_verdict_required_gate_payload_contract_rows_present_contract_v11',
      ok: requiredGatePayloadContractIdsMissingV9.length === 0,
      detail:
        requiredGatePayloadContractIdsMissingV9.length === 0
          ? 'ok'
          : requiredGatePayloadContractIdsMissingV9.join(','),
    },
    {
      id: 'release_verdict_v10_contract_rows_present_contract_v11',
      ok: v10ContractIdsMissing.length === 0,
      detail: v10ContractIdsMissing.length === 0 ? 'ok' : v10ContractIdsMissing.join(','),
    },
    {
      id: 'release_verdict_check_ids_unique_post_v10_contract_v11',
      ok: postV10CheckIdsDuplicate.length === 0,
      detail:
        postV10CheckIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(postV10CheckIdsDuplicate)).join(','),
    },
  );
  const postV11CheckIds = checks.map((row) => cleanText(row.id || '', 260)).filter(Boolean);
  const postV11CheckIdsDuplicate = postV11CheckIds.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const postV11CheckRowsObjectInvalidCount = checks.filter((row) => !isObjectRecord(row)).length;
  const postV11CheckIdTokenInvalid = checks
    .map((row) => cleanText((row as any)?.id || '', 260))
    .filter((id) => !id || !checkIdTokenRegex.test(id));
  const postV11CheckOkBooleanInvalid = checks
    .filter((row: any) => typeof row?.ok !== 'boolean')
    .map((row) => cleanText((row as any)?.id || '', 260));
  const postV11CheckDetailInvalid = checks
    .filter((row: any) => typeof row?.detail !== 'string' || !row.detail.trim())
    .map((row) => cleanText((row as any)?.id || '', 260));
  const postV11FailedIds = checks
    .filter((row) => !row.ok)
    .map((row) => cleanText(row.id || '', 260))
    .filter(Boolean);
  const postV11FailedRows = checks.filter((row) => !row.ok);
  const postV11FailedIdsTokenInvalid = postV11FailedIds.filter(
    (id) => !checkIdTokenRegex.test(id),
  );
  const postV11FailedIdsDuplicate = postV11FailedIds.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const postV11FailedIdsUnknown = postV11FailedIds.filter((id) => !postV11CheckIds.includes(id));
  const postV11FailedIdsOrderMatchesChecks =
    postV11FailedIds.join(',') ===
    checks
      .filter((row) => !row.ok)
      .map((row) => cleanText(row.id || '', 260))
      .join(',');
  const postV11FailedCountLeqCheckCount = postV11FailedIds.length <= postV11CheckIds.length;
  const postV11ReleaseReadyCandidate = postV11FailedRows.length === 0;
  const postV11ReleaseReadyConsistent =
    postV11ReleaseReadyCandidate === (postV11FailedIds.length === 0);
  const runtimeProofRowsV9Found = postV11CheckIds.filter((id) =>
    runtimeProofContractIdsV9.includes(id as any),
  );
  const runtimeProofRowsV9Missing = runtimeProofContractIdsV9.filter(
    (id) => !runtimeProofRowsV9Found.includes(id),
  );
  const runtimeProofRowsV9Unexpected = runtimeProofRowsV9Found.filter(
    (id) => !runtimeProofContractIdsV9.includes(id as any),
  );
  const runtimeProofRowsV9Duplicate = runtimeProofRowsV9Found.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const proofPackRowsV9Found = postV11CheckIds.filter((id) =>
    proofPackContractIdsV9.includes(id as any),
  );
  const proofPackRowsV9Missing = proofPackContractIdsV9.filter(
    (id) => !proofPackRowsV9Found.includes(id),
  );
  const proofPackRowsV9Unexpected = proofPackRowsV9Found.filter(
    (id) => !proofPackContractIdsV9.includes(id as any),
  );
  const proofPackRowsV9Duplicate = proofPackRowsV9Found.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const requiredGatePayloadRowsV9Found = postV11CheckIds.filter((id) =>
    requiredGatePayloadContractIdsV9.includes(id as any),
  );
  const requiredGatePayloadRowsV9Missing = requiredGatePayloadContractIdsV9.filter(
    (id) => !requiredGatePayloadRowsV9Found.includes(id),
  );
  const requiredGatePayloadRowsV9Unexpected = requiredGatePayloadRowsV9Found.filter(
    (id) => !requiredGatePayloadContractIdsV9.includes(id as any),
  );
  const requiredGatePayloadRowsV9Duplicate = requiredGatePayloadRowsV9Found.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const v10RowsFound = postV11CheckIds.filter((id) => v10ContractIds.includes(id as any));
  const v10RowsMissing = v10ContractIds.filter((id) => !v10RowsFound.includes(id));
  const v10RowsUnexpected = v10RowsFound.filter((id) => !v10ContractIds.includes(id as any));
  const v10RowsDuplicate = v10RowsFound.filter((id, index, arr) => id && arr.indexOf(id) !== index);
  const v11ContractIdsExpected = [
    'release_verdict_required_gate_step_rows_count_contract_v11',
    'release_verdict_required_gate_step_rows_unique_contract_v11',
    'release_verdict_required_gate_step_rows_set_contract_v11',
    'release_verdict_required_gate_step_rows_detail_contract_v11',
    'release_verdict_required_gate_artifact_rows_count_contract_v11',
    'release_verdict_required_gate_artifact_rows_unique_contract_v11',
    'release_verdict_required_gate_artifact_rows_set_contract_v11',
    'release_verdict_required_gate_artifact_rows_detail_contract_v11',
    'release_verdict_required_gate_health_rows_count_contract_v11',
    'release_verdict_required_gate_health_rows_unique_contract_v11',
    'release_verdict_required_gate_health_rows_set_contract_v11',
    'release_verdict_required_gate_health_rows_detail_contract_v11',
    'release_verdict_required_gate_strict_rows_count_contract_v11',
    'release_verdict_required_gate_strict_rows_set_contract_v11',
    'release_verdict_required_gate_strict_rows_detail_contract_v11',
    'release_verdict_runtime_proof_contract_rows_present_contract_v11',
    'release_verdict_proof_pack_contract_rows_present_contract_v11',
    'release_verdict_required_gate_payload_contract_rows_present_contract_v11',
    'release_verdict_v10_contract_rows_present_contract_v11',
    'release_verdict_check_ids_unique_post_v10_contract_v11',
  ];
  const v11RowsFound = postV11CheckIds.filter((id) => v11ContractIdsExpected.includes(id as any));
  const v11RowsMissing = v11ContractIdsExpected.filter((id) => !v11RowsFound.includes(id));
  const v11RowsUnexpected = v11RowsFound.filter((id) => !v11ContractIdsExpected.includes(id as any));
  const v11RowsDuplicate = v11RowsFound.filter((id, index, arr) => id && arr.indexOf(id) !== index);
  checks.push(
    {
      id: 'release_verdict_check_ids_unique_post_v11_contract_v12',
      ok: postV11CheckIdsDuplicate.length === 0,
      detail:
        postV11CheckIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(postV11CheckIdsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_check_rows_object_post_v11_contract_v12',
      ok: postV11CheckRowsObjectInvalidCount === 0,
      detail: `invalid_rows=${postV11CheckRowsObjectInvalidCount}`,
    },
    {
      id: 'release_verdict_check_id_token_post_v11_contract_v12',
      ok: postV11CheckIdTokenInvalid.length === 0,
      detail:
        postV11CheckIdTokenInvalid.length === 0 ? 'ok' : postV11CheckIdTokenInvalid.join(','),
    },
    {
      id: 'release_verdict_check_ok_boolean_post_v11_contract_v12',
      ok: postV11CheckOkBooleanInvalid.length === 0,
      detail:
        postV11CheckOkBooleanInvalid.length === 0 ? 'ok' : postV11CheckOkBooleanInvalid.join(','),
    },
    {
      id: 'release_verdict_check_detail_post_v11_contract_v12',
      ok: postV11CheckDetailInvalid.length === 0,
      detail:
        postV11CheckDetailInvalid.length === 0 ? 'ok' : postV11CheckDetailInvalid.join(','),
    },
    {
      id: 'release_verdict_failed_ids_token_post_v11_contract_v12',
      ok: postV11FailedIdsTokenInvalid.length === 0,
      detail:
        postV11FailedIdsTokenInvalid.length === 0
          ? 'ok'
          : postV11FailedIdsTokenInvalid.join(','),
    },
    {
      id: 'release_verdict_failed_ids_unique_post_v11_contract_v12',
      ok: postV11FailedIdsDuplicate.length === 0,
      detail:
        postV11FailedIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(postV11FailedIdsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_failed_ids_subset_post_v11_contract_v12',
      ok: postV11FailedIdsUnknown.length === 0,
      detail:
        postV11FailedIdsUnknown.length === 0 ? 'ok' : postV11FailedIdsUnknown.join(','),
    },
    {
      id: 'release_verdict_failed_ids_order_post_v11_contract_v12',
      ok: postV11FailedIdsOrderMatchesChecks,
      detail: `order_match=${String(postV11FailedIdsOrderMatchesChecks)}`,
    },
    {
      id: 'release_verdict_failed_count_leq_check_count_post_v11_contract_v12',
      ok: postV11FailedCountLeqCheckCount,
      detail: `failed=${postV11FailedIds.length};checks=${postV11CheckIds.length}`,
    },
    {
      id: 'release_verdict_release_ready_semantics_post_v11_contract_v12',
      ok: postV11ReleaseReadyConsistent,
      detail: `failed_rows=${postV11FailedRows.length};failed_ids=${postV11FailedIds.length};release_ready=${String(postV11ReleaseReadyCandidate)}`,
    },
    {
      id: 'release_verdict_runtime_proof_v9_rows_unique_contract_v12',
      ok: runtimeProofRowsV9Duplicate.length === 0,
      detail:
        runtimeProofRowsV9Duplicate.length === 0
          ? 'ok'
          : Array.from(new Set(runtimeProofRowsV9Duplicate)).join(','),
    },
    {
      id: 'release_verdict_runtime_proof_v9_rows_set_contract_v12',
      ok: runtimeProofRowsV9Missing.length === 0 && runtimeProofRowsV9Unexpected.length === 0,
      detail:
        runtimeProofRowsV9Missing.length === 0 && runtimeProofRowsV9Unexpected.length === 0
          ? 'ok'
          : `missing=${runtimeProofRowsV9Missing.join(',') || 'none'};unexpected=${runtimeProofRowsV9Unexpected.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_proof_pack_v9_rows_unique_contract_v12',
      ok: proofPackRowsV9Duplicate.length === 0,
      detail:
        proofPackRowsV9Duplicate.length === 0
          ? 'ok'
          : Array.from(new Set(proofPackRowsV9Duplicate)).join(','),
    },
    {
      id: 'release_verdict_proof_pack_v9_rows_set_contract_v12',
      ok: proofPackRowsV9Missing.length === 0 && proofPackRowsV9Unexpected.length === 0,
      detail:
        proofPackRowsV9Missing.length === 0 && proofPackRowsV9Unexpected.length === 0
          ? 'ok'
          : `missing=${proofPackRowsV9Missing.join(',') || 'none'};unexpected=${proofPackRowsV9Unexpected.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_required_gate_payload_v9_rows_unique_contract_v12',
      ok: requiredGatePayloadRowsV9Duplicate.length === 0,
      detail:
        requiredGatePayloadRowsV9Duplicate.length === 0
          ? 'ok'
          : Array.from(new Set(requiredGatePayloadRowsV9Duplicate)).join(','),
    },
    {
      id: 'release_verdict_required_gate_payload_v9_rows_set_contract_v12',
      ok:
        requiredGatePayloadRowsV9Missing.length === 0 &&
        requiredGatePayloadRowsV9Unexpected.length === 0,
      detail:
        requiredGatePayloadRowsV9Missing.length === 0 &&
        requiredGatePayloadRowsV9Unexpected.length === 0
          ? 'ok'
          : `missing=${requiredGatePayloadRowsV9Missing.join(',') || 'none'};unexpected=${requiredGatePayloadRowsV9Unexpected.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_v10_rows_unique_contract_v12',
      ok: v10RowsDuplicate.length === 0,
      detail:
        v10RowsDuplicate.length === 0 ? 'ok' : Array.from(new Set(v10RowsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_v10_rows_set_contract_v12',
      ok: v10RowsMissing.length === 0 && v10RowsUnexpected.length === 0,
      detail:
        v10RowsMissing.length === 0 && v10RowsUnexpected.length === 0
          ? 'ok'
          : `missing=${v10RowsMissing.join(',') || 'none'};unexpected=${v10RowsUnexpected.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_v11_rows_set_contract_v12',
      ok: v11RowsMissing.length === 0 && v11RowsUnexpected.length === 0 && v11RowsDuplicate.length === 0,
      detail:
        v11RowsMissing.length === 0 &&
        v11RowsUnexpected.length === 0 &&
        v11RowsDuplicate.length === 0
          ? 'ok'
          : `missing=${v11RowsMissing.join(',') || 'none'};unexpected=${v11RowsUnexpected.join(',') || 'none'};duplicate=${Array.from(new Set(v11RowsDuplicate)).join(',') || 'none'}`,
    },
  );
  const postV12CheckIds = checks.map((row) => cleanText(row.id || '', 260)).filter(Boolean);
  const postV12CheckIdsDuplicate = postV12CheckIds.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const postV12CheckRowsObjectInvalidCount = checks.filter((row) => !isObjectRecord(row)).length;
  const postV12CheckIdTokenInvalid = checks
    .map((row) => cleanText((row as any)?.id || '', 260))
    .filter((id) => !id || !checkIdTokenRegex.test(id));
  const postV12CheckOkBooleanInvalid = checks
    .filter((row: any) => typeof row?.ok !== 'boolean')
    .map((row) => cleanText((row as any)?.id || '', 260));
  const postV12CheckDetailInvalid = checks
    .filter((row: any) => typeof row?.detail !== 'string' || !row.detail.trim())
    .map((row) => cleanText((row as any)?.id || '', 260));
  const postV12FailedIds = checks
    .filter((row) => !row.ok)
    .map((row) => cleanText(row.id || '', 260))
    .filter(Boolean);
  const postV12FailedRows = checks.filter((row) => !row.ok);
  const postV12FailedIdsTokenInvalid = postV12FailedIds.filter(
    (id) => !checkIdTokenRegex.test(id),
  );
  const postV12FailedIdsDuplicate = postV12FailedIds.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const postV12FailedIdsUnknown = postV12FailedIds.filter((id) => !postV12CheckIds.includes(id));
  const postV12FailedIdsOrderMatchesChecks =
    postV12FailedIds.join(',') ===
    checks
      .filter((row) => !row.ok)
      .map((row) => cleanText(row.id || '', 260))
      .join(',');
  const postV12ReleaseReadyCandidate = postV12FailedRows.length === 0;
  const postV12ReleaseReadyConsistent =
    postV12ReleaseReadyCandidate === (postV12FailedIds.length === 0);
  const runtimeProofRowsV9FoundPostV12 = postV12CheckIds.filter((id) =>
    runtimeProofContractIdsV9.includes(id as any),
  );
  const proofPackRowsV9FoundPostV12 = postV12CheckIds.filter((id) =>
    proofPackContractIdsV9.includes(id as any),
  );
  const requiredGatePayloadRowsV9FoundPostV12 = postV12CheckIds.filter((id) =>
    requiredGatePayloadContractIdsV9.includes(id as any),
  );
  const v10RowsFoundPostV12 = postV12CheckIds.filter((id) => v10ContractIds.includes(id as any));
  const v11RowsFoundPostV12 = postV12CheckIds.filter((id) =>
    v11ContractIdsExpected.includes(id as any),
  );
  const v11RowsOrderMatchesPostV12 =
    v11RowsFoundPostV12.join(',') === v11ContractIdsExpected.join(',');
  const v12ContractIdsExpected = [
    'release_verdict_check_ids_unique_post_v11_contract_v12',
    'release_verdict_check_rows_object_post_v11_contract_v12',
    'release_verdict_check_id_token_post_v11_contract_v12',
    'release_verdict_check_ok_boolean_post_v11_contract_v12',
    'release_verdict_check_detail_post_v11_contract_v12',
    'release_verdict_failed_ids_token_post_v11_contract_v12',
    'release_verdict_failed_ids_unique_post_v11_contract_v12',
    'release_verdict_failed_ids_subset_post_v11_contract_v12',
    'release_verdict_failed_ids_order_post_v11_contract_v12',
    'release_verdict_failed_count_leq_check_count_post_v11_contract_v12',
    'release_verdict_release_ready_semantics_post_v11_contract_v12',
    'release_verdict_runtime_proof_v9_rows_unique_contract_v12',
    'release_verdict_runtime_proof_v9_rows_set_contract_v12',
    'release_verdict_proof_pack_v9_rows_unique_contract_v12',
    'release_verdict_proof_pack_v9_rows_set_contract_v12',
    'release_verdict_required_gate_payload_v9_rows_unique_contract_v12',
    'release_verdict_required_gate_payload_v9_rows_set_contract_v12',
    'release_verdict_v10_rows_unique_contract_v12',
    'release_verdict_v10_rows_set_contract_v12',
    'release_verdict_v11_rows_set_contract_v12',
  ];
  const v12RowsFound = postV12CheckIds.filter((id) => v12ContractIdsExpected.includes(id as any));
  const v12RowsMissing = v12ContractIdsExpected.filter((id) => !v12RowsFound.includes(id));
  const v12RowsUnexpected = v12RowsFound.filter((id) => !v12ContractIdsExpected.includes(id as any));
  const v12RowsDuplicate = v12RowsFound.filter((id, index, arr) => id && arr.indexOf(id) !== index);
  const v12RowsOrderMatches = v12RowsFound.join(',') === v12ContractIdsExpected.join(',');
  checks.push(
    {
      id: 'release_verdict_check_ids_unique_post_v12_contract_v13',
      ok: postV12CheckIdsDuplicate.length === 0,
      detail:
        postV12CheckIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(postV12CheckIdsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_check_rows_object_post_v12_contract_v13',
      ok: postV12CheckRowsObjectInvalidCount === 0,
      detail: `invalid_rows=${postV12CheckRowsObjectInvalidCount}`,
    },
    {
      id: 'release_verdict_check_id_token_post_v12_contract_v13',
      ok: postV12CheckIdTokenInvalid.length === 0,
      detail:
        postV12CheckIdTokenInvalid.length === 0 ? 'ok' : postV12CheckIdTokenInvalid.join(','),
    },
    {
      id: 'release_verdict_check_ok_boolean_post_v12_contract_v13',
      ok: postV12CheckOkBooleanInvalid.length === 0,
      detail:
        postV12CheckOkBooleanInvalid.length === 0 ? 'ok' : postV12CheckOkBooleanInvalid.join(','),
    },
    {
      id: 'release_verdict_check_detail_post_v12_contract_v13',
      ok: postV12CheckDetailInvalid.length === 0,
      detail:
        postV12CheckDetailInvalid.length === 0 ? 'ok' : postV12CheckDetailInvalid.join(','),
    },
    {
      id: 'release_verdict_failed_ids_token_post_v12_contract_v13',
      ok: postV12FailedIdsTokenInvalid.length === 0,
      detail:
        postV12FailedIdsTokenInvalid.length === 0
          ? 'ok'
          : postV12FailedIdsTokenInvalid.join(','),
    },
    {
      id: 'release_verdict_failed_ids_unique_post_v12_contract_v13',
      ok: postV12FailedIdsDuplicate.length === 0,
      detail:
        postV12FailedIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(postV12FailedIdsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_failed_ids_subset_post_v12_contract_v13',
      ok: postV12FailedIdsUnknown.length === 0,
      detail:
        postV12FailedIdsUnknown.length === 0 ? 'ok' : postV12FailedIdsUnknown.join(','),
    },
    {
      id: 'release_verdict_failed_ids_order_post_v12_contract_v13',
      ok: postV12FailedIdsOrderMatchesChecks,
      detail: `order_match=${String(postV12FailedIdsOrderMatchesChecks)}`,
    },
    {
      id: 'release_verdict_release_ready_semantics_post_v12_contract_v13',
      ok: postV12ReleaseReadyConsistent,
      detail: `failed_rows=${postV12FailedRows.length};failed_ids=${postV12FailedIds.length};release_ready=${String(postV12ReleaseReadyCandidate)}`,
    },
    {
      id: 'release_verdict_runtime_proof_v9_rows_count_contract_v13',
      ok: runtimeProofRowsV9FoundPostV12.length === runtimeProofContractIdsV9.length,
      detail: `rows=${runtimeProofRowsV9FoundPostV12.length};required=${runtimeProofContractIdsV9.length}`,
    },
    {
      id: 'release_verdict_proof_pack_v9_rows_count_contract_v13',
      ok: proofPackRowsV9FoundPostV12.length === proofPackContractIdsV9.length,
      detail: `rows=${proofPackRowsV9FoundPostV12.length};required=${proofPackContractIdsV9.length}`,
    },
    {
      id: 'release_verdict_required_gate_payload_v9_rows_count_contract_v13',
      ok: requiredGatePayloadRowsV9FoundPostV12.length === requiredGatePayloadContractIdsV9.length,
      detail: `rows=${requiredGatePayloadRowsV9FoundPostV12.length};required=${requiredGatePayloadContractIdsV9.length}`,
    },
    {
      id: 'release_verdict_v10_rows_count_contract_v13',
      ok: v10RowsFoundPostV12.length === v10ContractIds.length,
      detail: `rows=${v10RowsFoundPostV12.length};required=${v10ContractIds.length}`,
    },
    {
      id: 'release_verdict_v11_rows_count_contract_v13',
      ok: v11RowsFoundPostV12.length === v11ContractIdsExpected.length,
      detail: `rows=${v11RowsFoundPostV12.length};required=${v11ContractIdsExpected.length}`,
    },
    {
      id: 'release_verdict_v11_rows_order_contract_v13',
      ok: v11RowsOrderMatchesPostV12,
      detail: `order_match=${String(v11RowsOrderMatchesPostV12)}`,
    },
    {
      id: 'release_verdict_v12_rows_unique_contract_v13',
      ok: v12RowsDuplicate.length === 0,
      detail:
        v12RowsDuplicate.length === 0 ? 'ok' : Array.from(new Set(v12RowsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_v12_rows_set_contract_v13',
      ok: v12RowsMissing.length === 0 && v12RowsUnexpected.length === 0,
      detail:
        v12RowsMissing.length === 0 && v12RowsUnexpected.length === 0
          ? 'ok'
          : `missing=${v12RowsMissing.join(',') || 'none'};unexpected=${v12RowsUnexpected.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_v12_rows_count_contract_v13',
      ok: v12RowsFound.length === v12ContractIdsExpected.length,
      detail: `rows=${v12RowsFound.length};required=${v12ContractIdsExpected.length}`,
    },
    {
      id: 'release_verdict_v12_rows_order_contract_v13',
      ok: v12RowsOrderMatches,
      detail: `order_match=${String(v12RowsOrderMatches)}`,
    },
  );
  const postV13CheckIds = checks.map((row) => cleanText(row.id || '', 260)).filter(Boolean);
  const postV13CheckIdsDuplicate = postV13CheckIds.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const postV13CheckRowsObjectInvalidCount = checks.filter((row) => !isObjectRecord(row)).length;
  const postV13CheckIdTokenInvalid = checks
    .map((row) => cleanText((row as any)?.id || '', 260))
    .filter((id) => !id || !checkIdTokenRegex.test(id));
  const postV13CheckOkBooleanInvalid = checks
    .filter((row: any) => typeof row?.ok !== 'boolean')
    .map((row) => cleanText((row as any)?.id || '', 260));
  const postV13CheckDetailInvalid = checks
    .filter((row: any) => typeof row?.detail !== 'string' || !row.detail.trim())
    .map((row) => cleanText((row as any)?.id || '', 260));
  const postV13FailedIds = checks
    .filter((row) => !row.ok)
    .map((row) => cleanText(row.id || '', 260))
    .filter(Boolean);
  const postV13FailedRows = checks.filter((row) => !row.ok);
  const postV13FailedIdsTokenInvalid = postV13FailedIds.filter(
    (id) => !checkIdTokenRegex.test(id),
  );
  const postV13FailedIdsDuplicate = postV13FailedIds.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const postV13FailedIdsUnknown = postV13FailedIds.filter((id) => !postV13CheckIds.includes(id));
  const postV13FailedIdsOrderMatchesChecks =
    postV13FailedIds.join(',') ===
    checks
      .filter((row) => !row.ok)
      .map((row) => cleanText(row.id || '', 260))
      .join(',');
  const postV13ReleaseReadyCandidate = postV13FailedRows.length === 0;
  const postV13ReleaseReadyConsistent =
    postV13ReleaseReadyCandidate === (postV13FailedIds.length === 0);
  const runtimeProofRowsV9FoundPostV13 = postV13CheckIds.filter((id) =>
    runtimeProofContractIdsV9.includes(id as any),
  );
  const runtimeProofRowsV9MissingPostV13 = runtimeProofContractIdsV9.filter(
    (id) => !runtimeProofRowsV9FoundPostV13.includes(id),
  );
  const runtimeProofRowsV9UnexpectedPostV13 = runtimeProofRowsV9FoundPostV13.filter(
    (id) => !runtimeProofContractIdsV9.includes(id as any),
  );
  const runtimeProofRowsV9DuplicatePostV13 = runtimeProofRowsV9FoundPostV13.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const proofPackRowsV9FoundPostV13 = postV13CheckIds.filter((id) =>
    proofPackContractIdsV9.includes(id as any),
  );
  const proofPackRowsV9MissingPostV13 = proofPackContractIdsV9.filter(
    (id) => !proofPackRowsV9FoundPostV13.includes(id),
  );
  const proofPackRowsV9UnexpectedPostV13 = proofPackRowsV9FoundPostV13.filter(
    (id) => !proofPackContractIdsV9.includes(id as any),
  );
  const proofPackRowsV9DuplicatePostV13 = proofPackRowsV9FoundPostV13.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const requiredGatePayloadRowsV9FoundPostV13 = postV13CheckIds.filter((id) =>
    requiredGatePayloadContractIdsV9.includes(id as any),
  );
  const requiredGatePayloadRowsV9MissingPostV13 = requiredGatePayloadContractIdsV9.filter(
    (id) => !requiredGatePayloadRowsV9FoundPostV13.includes(id),
  );
  const requiredGatePayloadRowsV9UnexpectedPostV13 = requiredGatePayloadRowsV9FoundPostV13.filter(
    (id) => !requiredGatePayloadContractIdsV9.includes(id as any),
  );
  const requiredGatePayloadRowsV9DuplicatePostV13 = requiredGatePayloadRowsV9FoundPostV13.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const v10RowsFoundPostV13 = postV13CheckIds.filter((id) => v10ContractIds.includes(id as any));
  const v10RowsMissingPostV13 = v10ContractIds.filter((id) => !v10RowsFoundPostV13.includes(id));
  const v10RowsUnexpectedPostV13 = v10RowsFoundPostV13.filter(
    (id) => !v10ContractIds.includes(id as any),
  );
  const v10RowsDuplicatePostV13 = v10RowsFoundPostV13.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const v13ContractIdsExpected = [
    'release_verdict_check_ids_unique_post_v12_contract_v13',
    'release_verdict_check_rows_object_post_v12_contract_v13',
    'release_verdict_check_id_token_post_v12_contract_v13',
    'release_verdict_check_ok_boolean_post_v12_contract_v13',
    'release_verdict_check_detail_post_v12_contract_v13',
    'release_verdict_failed_ids_token_post_v12_contract_v13',
    'release_verdict_failed_ids_unique_post_v12_contract_v13',
    'release_verdict_failed_ids_subset_post_v12_contract_v13',
    'release_verdict_failed_ids_order_post_v12_contract_v13',
    'release_verdict_release_ready_semantics_post_v12_contract_v13',
    'release_verdict_runtime_proof_v9_rows_count_contract_v13',
    'release_verdict_proof_pack_v9_rows_count_contract_v13',
    'release_verdict_required_gate_payload_v9_rows_count_contract_v13',
    'release_verdict_v10_rows_count_contract_v13',
    'release_verdict_v11_rows_count_contract_v13',
    'release_verdict_v11_rows_order_contract_v13',
    'release_verdict_v12_rows_unique_contract_v13',
    'release_verdict_v12_rows_set_contract_v13',
    'release_verdict_v12_rows_count_contract_v13',
    'release_verdict_v12_rows_order_contract_v13',
  ];
  const v13RowsFound = postV13CheckIds.filter((id) => v13ContractIdsExpected.includes(id as any));
  const v13RowsMissing = v13ContractIdsExpected.filter((id) => !v13RowsFound.includes(id));
  const v13RowsUnexpected = v13RowsFound.filter((id) => !v13ContractIdsExpected.includes(id as any));
  const v13RowsDuplicate = v13RowsFound.filter((id, index, arr) => id && arr.indexOf(id) !== index);
  const v13RowsOrderMatches = v13RowsFound.join(',') === v13ContractIdsExpected.join(',');
  checks.push(
    {
      id: 'release_verdict_check_ids_unique_post_v13_contract_v14',
      ok: postV13CheckIdsDuplicate.length === 0,
      detail:
        postV13CheckIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(postV13CheckIdsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_check_rows_object_post_v13_contract_v14',
      ok: postV13CheckRowsObjectInvalidCount === 0,
      detail: `invalid_rows=${postV13CheckRowsObjectInvalidCount}`,
    },
    {
      id: 'release_verdict_check_id_token_post_v13_contract_v14',
      ok: postV13CheckIdTokenInvalid.length === 0,
      detail:
        postV13CheckIdTokenInvalid.length === 0 ? 'ok' : postV13CheckIdTokenInvalid.join(','),
    },
    {
      id: 'release_verdict_check_ok_boolean_post_v13_contract_v14',
      ok: postV13CheckOkBooleanInvalid.length === 0,
      detail:
        postV13CheckOkBooleanInvalid.length === 0 ? 'ok' : postV13CheckOkBooleanInvalid.join(','),
    },
    {
      id: 'release_verdict_check_detail_post_v13_contract_v14',
      ok: postV13CheckDetailInvalid.length === 0,
      detail:
        postV13CheckDetailInvalid.length === 0 ? 'ok' : postV13CheckDetailInvalid.join(','),
    },
    {
      id: 'release_verdict_failed_ids_token_post_v13_contract_v14',
      ok: postV13FailedIdsTokenInvalid.length === 0,
      detail:
        postV13FailedIdsTokenInvalid.length === 0
          ? 'ok'
          : postV13FailedIdsTokenInvalid.join(','),
    },
    {
      id: 'release_verdict_failed_ids_unique_post_v13_contract_v14',
      ok: postV13FailedIdsDuplicate.length === 0,
      detail:
        postV13FailedIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(postV13FailedIdsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_failed_ids_subset_post_v13_contract_v14',
      ok: postV13FailedIdsUnknown.length === 0,
      detail:
        postV13FailedIdsUnknown.length === 0 ? 'ok' : postV13FailedIdsUnknown.join(','),
    },
    {
      id: 'release_verdict_failed_ids_order_post_v13_contract_v14',
      ok: postV13FailedIdsOrderMatchesChecks,
      detail: `order_match=${String(postV13FailedIdsOrderMatchesChecks)}`,
    },
    {
      id: 'release_verdict_release_ready_semantics_post_v13_contract_v14',
      ok: postV13ReleaseReadyConsistent,
      detail: `failed_rows=${postV13FailedRows.length};failed_ids=${postV13FailedIds.length};release_ready=${String(postV13ReleaseReadyCandidate)}`,
    },
    {
      id: 'release_verdict_runtime_proof_v9_rows_unique_post_v13_contract_v14',
      ok: runtimeProofRowsV9DuplicatePostV13.length === 0,
      detail:
        runtimeProofRowsV9DuplicatePostV13.length === 0
          ? 'ok'
          : Array.from(new Set(runtimeProofRowsV9DuplicatePostV13)).join(','),
    },
    {
      id: 'release_verdict_runtime_proof_v9_rows_set_post_v13_contract_v14',
      ok:
        runtimeProofRowsV9MissingPostV13.length === 0 &&
        runtimeProofRowsV9UnexpectedPostV13.length === 0,
      detail:
        runtimeProofRowsV9MissingPostV13.length === 0 &&
        runtimeProofRowsV9UnexpectedPostV13.length === 0
          ? 'ok'
          : `missing=${runtimeProofRowsV9MissingPostV13.join(',') || 'none'};unexpected=${runtimeProofRowsV9UnexpectedPostV13.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_proof_pack_v9_rows_unique_post_v13_contract_v14',
      ok: proofPackRowsV9DuplicatePostV13.length === 0,
      detail:
        proofPackRowsV9DuplicatePostV13.length === 0
          ? 'ok'
          : Array.from(new Set(proofPackRowsV9DuplicatePostV13)).join(','),
    },
    {
      id: 'release_verdict_proof_pack_v9_rows_set_post_v13_contract_v14',
      ok: proofPackRowsV9MissingPostV13.length === 0 && proofPackRowsV9UnexpectedPostV13.length === 0,
      detail:
        proofPackRowsV9MissingPostV13.length === 0 && proofPackRowsV9UnexpectedPostV13.length === 0
          ? 'ok'
          : `missing=${proofPackRowsV9MissingPostV13.join(',') || 'none'};unexpected=${proofPackRowsV9UnexpectedPostV13.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_required_gate_payload_v9_rows_unique_post_v13_contract_v14',
      ok: requiredGatePayloadRowsV9DuplicatePostV13.length === 0,
      detail:
        requiredGatePayloadRowsV9DuplicatePostV13.length === 0
          ? 'ok'
          : Array.from(new Set(requiredGatePayloadRowsV9DuplicatePostV13)).join(','),
    },
    {
      id: 'release_verdict_required_gate_payload_v9_rows_set_post_v13_contract_v14',
      ok:
        requiredGatePayloadRowsV9MissingPostV13.length === 0 &&
        requiredGatePayloadRowsV9UnexpectedPostV13.length === 0,
      detail:
        requiredGatePayloadRowsV9MissingPostV13.length === 0 &&
        requiredGatePayloadRowsV9UnexpectedPostV13.length === 0
          ? 'ok'
          : `missing=${requiredGatePayloadRowsV9MissingPostV13.join(',') || 'none'};unexpected=${requiredGatePayloadRowsV9UnexpectedPostV13.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_v10_rows_unique_post_v13_contract_v14',
      ok: v10RowsDuplicatePostV13.length === 0,
      detail:
        v10RowsDuplicatePostV13.length === 0
          ? 'ok'
          : Array.from(new Set(v10RowsDuplicatePostV13)).join(','),
    },
    {
      id: 'release_verdict_v10_rows_set_post_v13_contract_v14',
      ok: v10RowsMissingPostV13.length === 0 && v10RowsUnexpectedPostV13.length === 0,
      detail:
        v10RowsMissingPostV13.length === 0 && v10RowsUnexpectedPostV13.length === 0
          ? 'ok'
          : `missing=${v10RowsMissingPostV13.join(',') || 'none'};unexpected=${v10RowsUnexpectedPostV13.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_v13_rows_unique_contract_v14',
      ok: v13RowsDuplicate.length === 0,
      detail:
        v13RowsDuplicate.length === 0 ? 'ok' : Array.from(new Set(v13RowsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_v13_rows_set_and_order_contract_v14',
      ok: v13RowsMissing.length === 0 && v13RowsUnexpected.length === 0 && v13RowsOrderMatches,
      detail:
        v13RowsMissing.length === 0 && v13RowsUnexpected.length === 0 && v13RowsOrderMatches
          ? 'ok'
          : `missing=${v13RowsMissing.join(',') || 'none'};unexpected=${v13RowsUnexpected.join(',') || 'none'};order_match=${String(v13RowsOrderMatches)}`,
    },
  );
  const postV14CheckIds = checks.map((row) => cleanText(row.id || '', 260)).filter(Boolean);
  const postV14CheckIdsDuplicate = postV14CheckIds.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const postV14CheckRowsObjectInvalidCount = checks.filter((row) => !isObjectRecord(row)).length;
  const postV14CheckIdTokenInvalid = checks
    .map((row) => cleanText((row as any)?.id || '', 260))
    .filter((id) => !id || !checkIdTokenRegex.test(id));
  const postV14CheckOkBooleanInvalid = checks
    .filter((row: any) => typeof row?.ok !== 'boolean')
    .map((row) => cleanText((row as any)?.id || '', 260));
  const postV14CheckDetailInvalid = checks
    .filter((row: any) => typeof row?.detail !== 'string' || !row.detail.trim())
    .map((row) => cleanText((row as any)?.id || '', 260));
  const postV14FailedIds = checks
    .filter((row) => !row.ok)
    .map((row) => cleanText(row.id || '', 260))
    .filter(Boolean);
  const postV14FailedRows = checks.filter((row) => !row.ok);
  const postV14FailedIdsTokenInvalid = postV14FailedIds.filter(
    (id) => !checkIdTokenRegex.test(id),
  );
  const postV14FailedIdsDuplicate = postV14FailedIds.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const postV14FailedIdsUnknown = postV14FailedIds.filter((id) => !postV14CheckIds.includes(id));
  const postV14FailedIdsOrderMatchesChecks =
    postV14FailedIds.join(',') ===
    checks
      .filter((row) => !row.ok)
      .map((row) => cleanText(row.id || '', 260))
      .join(',');
  const postV14ReleaseReadyCandidate = postV14FailedRows.length === 0;
  const postV14ReleaseReadyConsistent =
    postV14ReleaseReadyCandidate === (postV14FailedIds.length === 0);
  const runtimeProofRowsV9FoundPostV14 = postV14CheckIds.filter((id) =>
    runtimeProofContractIdsV9.includes(id as any),
  );
  const proofPackRowsV9FoundPostV14 = postV14CheckIds.filter((id) =>
    proofPackContractIdsV9.includes(id as any),
  );
  const requiredGatePayloadRowsV9FoundPostV14 = postV14CheckIds.filter((id) =>
    requiredGatePayloadContractIdsV9.includes(id as any),
  );
  const v10RowsFoundPostV14 = postV14CheckIds.filter((id) => v10ContractIds.includes(id as any));
  const v11RowsFoundPostV14 = postV14CheckIds.filter((id) =>
    v11ContractIdsExpected.includes(id as any),
  );
  const v12RowsFoundPostV14 = postV14CheckIds.filter((id) =>
    v12ContractIdsExpected.includes(id as any),
  );
  const v13RowsFoundPostV14 = postV14CheckIds.filter((id) =>
    v13ContractIdsExpected.includes(id as any),
  );
  const v14ContractIdsExpected = [
    'release_verdict_check_ids_unique_post_v13_contract_v14',
    'release_verdict_check_rows_object_post_v13_contract_v14',
    'release_verdict_check_id_token_post_v13_contract_v14',
    'release_verdict_check_ok_boolean_post_v13_contract_v14',
    'release_verdict_check_detail_post_v13_contract_v14',
    'release_verdict_failed_ids_token_post_v13_contract_v14',
    'release_verdict_failed_ids_unique_post_v13_contract_v14',
    'release_verdict_failed_ids_subset_post_v13_contract_v14',
    'release_verdict_failed_ids_order_post_v13_contract_v14',
    'release_verdict_release_ready_semantics_post_v13_contract_v14',
    'release_verdict_runtime_proof_v9_rows_unique_post_v13_contract_v14',
    'release_verdict_runtime_proof_v9_rows_set_post_v13_contract_v14',
    'release_verdict_proof_pack_v9_rows_unique_post_v13_contract_v14',
    'release_verdict_proof_pack_v9_rows_set_post_v13_contract_v14',
    'release_verdict_required_gate_payload_v9_rows_unique_post_v13_contract_v14',
    'release_verdict_required_gate_payload_v9_rows_set_post_v13_contract_v14',
    'release_verdict_v10_rows_unique_post_v13_contract_v14',
    'release_verdict_v10_rows_set_post_v13_contract_v14',
    'release_verdict_v13_rows_unique_contract_v14',
    'release_verdict_v13_rows_set_and_order_contract_v14',
  ];
  const v14RowsFound = postV14CheckIds.filter((id) => v14ContractIdsExpected.includes(id as any));
  const v14RowsMissing = v14ContractIdsExpected.filter((id) => !v14RowsFound.includes(id));
  const v14RowsUnexpected = v14RowsFound.filter((id) => !v14ContractIdsExpected.includes(id as any));
  const v14RowsDuplicate = v14RowsFound.filter((id, index, arr) => id && arr.indexOf(id) !== index);
  const v14RowsOrderMatches = v14RowsFound.join(',') === v14ContractIdsExpected.join(',');
  checks.push(
    {
      id: 'release_verdict_check_ids_unique_post_v14_contract_v15',
      ok: postV14CheckIdsDuplicate.length === 0,
      detail:
        postV14CheckIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(postV14CheckIdsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_check_rows_object_post_v14_contract_v15',
      ok: postV14CheckRowsObjectInvalidCount === 0,
      detail: `invalid_rows=${postV14CheckRowsObjectInvalidCount}`,
    },
    {
      id: 'release_verdict_check_id_token_post_v14_contract_v15',
      ok: postV14CheckIdTokenInvalid.length === 0,
      detail:
        postV14CheckIdTokenInvalid.length === 0 ? 'ok' : postV14CheckIdTokenInvalid.join(','),
    },
    {
      id: 'release_verdict_check_ok_boolean_post_v14_contract_v15',
      ok: postV14CheckOkBooleanInvalid.length === 0,
      detail:
        postV14CheckOkBooleanInvalid.length === 0 ? 'ok' : postV14CheckOkBooleanInvalid.join(','),
    },
    {
      id: 'release_verdict_check_detail_post_v14_contract_v15',
      ok: postV14CheckDetailInvalid.length === 0,
      detail:
        postV14CheckDetailInvalid.length === 0 ? 'ok' : postV14CheckDetailInvalid.join(','),
    },
    {
      id: 'release_verdict_failed_ids_token_post_v14_contract_v15',
      ok: postV14FailedIdsTokenInvalid.length === 0,
      detail:
        postV14FailedIdsTokenInvalid.length === 0
          ? 'ok'
          : postV14FailedIdsTokenInvalid.join(','),
    },
    {
      id: 'release_verdict_failed_ids_unique_post_v14_contract_v15',
      ok: postV14FailedIdsDuplicate.length === 0,
      detail:
        postV14FailedIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(postV14FailedIdsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_failed_ids_subset_post_v14_contract_v15',
      ok: postV14FailedIdsUnknown.length === 0,
      detail:
        postV14FailedIdsUnknown.length === 0 ? 'ok' : postV14FailedIdsUnknown.join(','),
    },
    {
      id: 'release_verdict_failed_ids_order_post_v14_contract_v15',
      ok: postV14FailedIdsOrderMatchesChecks,
      detail: `order_match=${String(postV14FailedIdsOrderMatchesChecks)}`,
    },
    {
      id: 'release_verdict_release_ready_semantics_post_v14_contract_v15',
      ok: postV14ReleaseReadyConsistent,
      detail: `failed_rows=${postV14FailedRows.length};failed_ids=${postV14FailedIds.length};release_ready=${String(postV14ReleaseReadyCandidate)}`,
    },
    {
      id: 'release_verdict_runtime_proof_v9_rows_count_post_v14_contract_v15',
      ok: runtimeProofRowsV9FoundPostV14.length === runtimeProofContractIdsV9.length,
      detail: `rows=${runtimeProofRowsV9FoundPostV14.length};required=${runtimeProofContractIdsV9.length}`,
    },
    {
      id: 'release_verdict_proof_pack_v9_rows_count_post_v14_contract_v15',
      ok: proofPackRowsV9FoundPostV14.length === proofPackContractIdsV9.length,
      detail: `rows=${proofPackRowsV9FoundPostV14.length};required=${proofPackContractIdsV9.length}`,
    },
    {
      id: 'release_verdict_required_gate_payload_v9_rows_count_post_v14_contract_v15',
      ok: requiredGatePayloadRowsV9FoundPostV14.length === requiredGatePayloadContractIdsV9.length,
      detail: `rows=${requiredGatePayloadRowsV9FoundPostV14.length};required=${requiredGatePayloadContractIdsV9.length}`,
    },
    {
      id: 'release_verdict_v10_rows_count_post_v14_contract_v15',
      ok: v10RowsFoundPostV14.length === v10ContractIds.length,
      detail: `rows=${v10RowsFoundPostV14.length};required=${v10ContractIds.length}`,
    },
    {
      id: 'release_verdict_v11_rows_count_post_v14_contract_v15',
      ok: v11RowsFoundPostV14.length === v11ContractIdsExpected.length,
      detail: `rows=${v11RowsFoundPostV14.length};required=${v11ContractIdsExpected.length}`,
    },
    {
      id: 'release_verdict_v12_rows_count_post_v14_contract_v15',
      ok: v12RowsFoundPostV14.length === v12ContractIdsExpected.length,
      detail: `rows=${v12RowsFoundPostV14.length};required=${v12ContractIdsExpected.length}`,
    },
    {
      id: 'release_verdict_v13_rows_count_post_v14_contract_v15',
      ok: v13RowsFoundPostV14.length === v13ContractIdsExpected.length,
      detail: `rows=${v13RowsFoundPostV14.length};required=${v13ContractIdsExpected.length}`,
    },
    {
      id: 'release_verdict_v14_rows_unique_contract_v15',
      ok: v14RowsDuplicate.length === 0,
      detail:
        v14RowsDuplicate.length === 0 ? 'ok' : Array.from(new Set(v14RowsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_v14_rows_set_contract_v15',
      ok: v14RowsMissing.length === 0 && v14RowsUnexpected.length === 0,
      detail:
        v14RowsMissing.length === 0 && v14RowsUnexpected.length === 0
          ? 'ok'
          : `missing=${v14RowsMissing.join(',') || 'none'};unexpected=${v14RowsUnexpected.join(',') || 'none'}`,
    },
    {
      id: 'release_verdict_v14_rows_order_contract_v15',
      ok: v14RowsOrderMatches,
      detail: `order_match=${String(v14RowsOrderMatches)}`,
    },
  );
  const postV15CheckIds = checks.map((row) => cleanText(row.id || '', 260)).filter(Boolean);
  const postV15CheckIdsDuplicate = postV15CheckIds.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const postV15CheckRowsObjectInvalidCount = checks.filter((row) => !isObjectRecord(row)).length;
  const postV15CheckIdTokenInvalid = checks
    .map((row) => cleanText((row as any)?.id || '', 260))
    .filter((id) => !id || !checkIdTokenRegex.test(id));
  const postV15CheckOkBooleanInvalid = checks
    .filter((row: any) => typeof row?.ok !== 'boolean')
    .map((row) => cleanText((row as any)?.id || '', 260));
  const postV15CheckDetailInvalid = checks
    .filter((row: any) => typeof row?.detail !== 'string' || !row.detail.trim())
    .map((row) => cleanText((row as any)?.id || '', 260));
  const postV15FailedIds = checks
    .filter((row) => !row.ok)
    .map((row) => cleanText(row.id || '', 260))
    .filter(Boolean);
  const postV15FailedRows = checks.filter((row) => !row.ok);
  const postV15FailedIdsTokenInvalid = postV15FailedIds.filter(
    (id) => !checkIdTokenRegex.test(id),
  );
  const postV15FailedIdsDuplicate = postV15FailedIds.filter(
    (id, index, arr) => id && arr.indexOf(id) !== index,
  );
  const postV15FailedIdsUnknown = postV15FailedIds.filter((id) => !postV15CheckIds.includes(id));
  const postV15FailedIdsOrderMatchesChecks =
    postV15FailedIds.join(',') ===
    checks
      .filter((row) => !row.ok)
      .map((row) => cleanText(row.id || '', 260))
      .join(',');
  const postV15ReleaseReadyCandidate = postV15FailedRows.length === 0;
  const postV15ReleaseReadyConsistent =
    postV15ReleaseReadyCandidate === (postV15FailedIds.length === 0);
  const runtimeProofRowsV9FoundPostV15 = postV15CheckIds.filter((id) =>
    runtimeProofContractIdsV9.includes(id as any),
  );
  const proofPackRowsV9FoundPostV15 = postV15CheckIds.filter((id) =>
    proofPackContractIdsV9.includes(id as any),
  );
  const requiredGatePayloadRowsV9FoundPostV15 = postV15CheckIds.filter((id) =>
    requiredGatePayloadContractIdsV9.includes(id as any),
  );
  const v10RowsFoundPostV15 = postV15CheckIds.filter((id) => v10ContractIds.includes(id as any));
  const v11RowsFoundPostV15 = postV15CheckIds.filter((id) =>
    v11ContractIdsExpected.includes(id as any),
  );
  const v12RowsFoundPostV15 = postV15CheckIds.filter((id) =>
    v12ContractIdsExpected.includes(id as any),
  );
  const v13RowsFoundPostV15 = postV15CheckIds.filter((id) =>
    v13ContractIdsExpected.includes(id as any),
  );
  const v14RowsFoundPostV15 = postV15CheckIds.filter((id) =>
    v14ContractIdsExpected.includes(id as any),
  );
  const v15ContractIdsExpected = [
    'release_verdict_check_ids_unique_post_v14_contract_v15',
    'release_verdict_check_rows_object_post_v14_contract_v15',
    'release_verdict_check_id_token_post_v14_contract_v15',
    'release_verdict_check_ok_boolean_post_v14_contract_v15',
    'release_verdict_check_detail_post_v14_contract_v15',
    'release_verdict_failed_ids_token_post_v14_contract_v15',
    'release_verdict_failed_ids_unique_post_v14_contract_v15',
    'release_verdict_failed_ids_subset_post_v14_contract_v15',
    'release_verdict_failed_ids_order_post_v14_contract_v15',
    'release_verdict_release_ready_semantics_post_v14_contract_v15',
    'release_verdict_runtime_proof_v9_rows_count_post_v14_contract_v15',
    'release_verdict_proof_pack_v9_rows_count_post_v14_contract_v15',
    'release_verdict_required_gate_payload_v9_rows_count_post_v14_contract_v15',
    'release_verdict_v10_rows_count_post_v14_contract_v15',
    'release_verdict_v11_rows_count_post_v14_contract_v15',
    'release_verdict_v12_rows_count_post_v14_contract_v15',
    'release_verdict_v13_rows_count_post_v14_contract_v15',
    'release_verdict_v14_rows_unique_contract_v15',
    'release_verdict_v14_rows_set_contract_v15',
    'release_verdict_v14_rows_order_contract_v15',
  ];
  const v15RowsFound = postV15CheckIds.filter((id) => v15ContractIdsExpected.includes(id as any));
  const v15RowsMissing = v15ContractIdsExpected.filter((id) => !v15RowsFound.includes(id));
  const v15RowsUnexpected = v15RowsFound.filter((id) => !v15ContractIdsExpected.includes(id as any));
  const v15RowsDuplicate = v15RowsFound.filter((id, index, arr) => id && arr.indexOf(id) !== index);
  const v15RowsOrderMatches = v15RowsFound.join(',') === v15ContractIdsExpected.join(',');
  checks.push(
    {
      id: 'release_verdict_check_ids_unique_post_v15_contract_v16',
      ok: postV15CheckIdsDuplicate.length === 0,
      detail:
        postV15CheckIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(postV15CheckIdsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_check_rows_object_post_v15_contract_v16',
      ok: postV15CheckRowsObjectInvalidCount === 0,
      detail: `invalid_rows=${postV15CheckRowsObjectInvalidCount}`,
    },
    {
      id: 'release_verdict_check_id_token_post_v15_contract_v16',
      ok: postV15CheckIdTokenInvalid.length === 0,
      detail:
        postV15CheckIdTokenInvalid.length === 0 ? 'ok' : postV15CheckIdTokenInvalid.join(','),
    },
    {
      id: 'release_verdict_check_ok_boolean_post_v15_contract_v16',
      ok: postV15CheckOkBooleanInvalid.length === 0,
      detail:
        postV15CheckOkBooleanInvalid.length === 0 ? 'ok' : postV15CheckOkBooleanInvalid.join(','),
    },
    {
      id: 'release_verdict_check_detail_post_v15_contract_v16',
      ok: postV15CheckDetailInvalid.length === 0,
      detail:
        postV15CheckDetailInvalid.length === 0 ? 'ok' : postV15CheckDetailInvalid.join(','),
    },
    {
      id: 'release_verdict_failed_ids_token_post_v15_contract_v16',
      ok: postV15FailedIdsTokenInvalid.length === 0,
      detail:
        postV15FailedIdsTokenInvalid.length === 0
          ? 'ok'
          : postV15FailedIdsTokenInvalid.join(','),
    },
    {
      id: 'release_verdict_failed_ids_unique_post_v15_contract_v16',
      ok: postV15FailedIdsDuplicate.length === 0,
      detail:
        postV15FailedIdsDuplicate.length === 0
          ? 'ok'
          : Array.from(new Set(postV15FailedIdsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_failed_ids_subset_post_v15_contract_v16',
      ok: postV15FailedIdsUnknown.length === 0,
      detail:
        postV15FailedIdsUnknown.length === 0 ? 'ok' : postV15FailedIdsUnknown.join(','),
    },
    {
      id: 'release_verdict_failed_ids_order_post_v15_contract_v16',
      ok: postV15FailedIdsOrderMatchesChecks,
      detail: `order_match=${String(postV15FailedIdsOrderMatchesChecks)}`,
    },
    {
      id: 'release_verdict_release_ready_semantics_post_v15_contract_v16',
      ok: postV15ReleaseReadyConsistent,
      detail: `failed_rows=${postV15FailedRows.length};failed_ids=${postV15FailedIds.length};release_ready=${String(postV15ReleaseReadyCandidate)}`,
    },
    {
      id: 'release_verdict_runtime_proof_v9_rows_count_post_v15_contract_v16',
      ok: runtimeProofRowsV9FoundPostV15.length === runtimeProofContractIdsV9.length,
      detail: `rows=${runtimeProofRowsV9FoundPostV15.length};required=${runtimeProofContractIdsV9.length}`,
    },
    {
      id: 'release_verdict_proof_pack_v9_rows_count_post_v15_contract_v16',
      ok: proofPackRowsV9FoundPostV15.length === proofPackContractIdsV9.length,
      detail: `rows=${proofPackRowsV9FoundPostV15.length};required=${proofPackContractIdsV9.length}`,
    },
    {
      id: 'release_verdict_required_gate_payload_v9_rows_count_post_v15_contract_v16',
      ok: requiredGatePayloadRowsV9FoundPostV15.length === requiredGatePayloadContractIdsV9.length,
      detail: `rows=${requiredGatePayloadRowsV9FoundPostV15.length};required=${requiredGatePayloadContractIdsV9.length}`,
    },
    {
      id: 'release_verdict_v10_rows_count_post_v15_contract_v16',
      ok: v10RowsFoundPostV15.length === v10ContractIds.length,
      detail: `rows=${v10RowsFoundPostV15.length};required=${v10ContractIds.length}`,
    },
    {
      id: 'release_verdict_v11_rows_count_post_v15_contract_v16',
      ok: v11RowsFoundPostV15.length === v11ContractIdsExpected.length,
      detail: `rows=${v11RowsFoundPostV15.length};required=${v11ContractIdsExpected.length}`,
    },
    {
      id: 'release_verdict_v12_rows_count_post_v15_contract_v16',
      ok: v12RowsFoundPostV15.length === v12ContractIdsExpected.length,
      detail: `rows=${v12RowsFoundPostV15.length};required=${v12ContractIdsExpected.length}`,
    },
    {
      id: 'release_verdict_v13_rows_count_post_v15_contract_v16',
      ok: v13RowsFoundPostV15.length === v13ContractIdsExpected.length,
      detail: `rows=${v13RowsFoundPostV15.length};required=${v13ContractIdsExpected.length}`,
    },
    {
      id: 'release_verdict_v14_rows_count_post_v15_contract_v16',
      ok: v14RowsFoundPostV15.length === v14ContractIdsExpected.length,
      detail: `rows=${v14RowsFoundPostV15.length};required=${v14ContractIdsExpected.length}`,
    },
    {
      id: 'release_verdict_v15_rows_unique_contract_v16',
      ok: v15RowsDuplicate.length === 0,
      detail:
        v15RowsDuplicate.length === 0 ? 'ok' : Array.from(new Set(v15RowsDuplicate)).join(','),
    },
    {
      id: 'release_verdict_v15_rows_set_and_order_contract_v16',
      ok: v15RowsMissing.length === 0 && v15RowsUnexpected.length === 0 && v15RowsOrderMatches,
      detail:
        v15RowsMissing.length === 0 && v15RowsUnexpected.length === 0 && v15RowsOrderMatches
          ? 'ok'
          : `missing=${v15RowsMissing.join(',') || 'none'};unexpected=${v15RowsUnexpected.join(',') || 'none'};order_match=${String(v15RowsOrderMatches)}`,
    },
  );
  const failed = checks.filter((row) => !row.ok);
  return {
    root,
    outPath: resolveMaybe(root, args.out || DEFAULT_OUT),
    report: {
      ok: failed.length === 0,
      type: 'release_verdict',
      generated_at: new Date().toISOString(),
      strict: Boolean(args.strict),
      summary: {
        check_count: checks.length,
        failed_count: failed.length,
        release_ready: failed.length === 0,
      },
      failed_ids: failed.map((row) => row.id),
      checks,
      artifact_hashes,
      verdict_checksum,
    },
  };
}

export function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const result = buildReport(args);
  return emitStructuredResult(result.report, {
    outPath: result.outPath,
    strict: args.strict,
    ok: result.report.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  buildReport,
  run,
};
