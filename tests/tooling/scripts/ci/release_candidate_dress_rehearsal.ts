#!/usr/bin/env node
'use strict';

import { parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult } from '../../lib/result.ts';
import { DEFAULT_GATE_REGISTRY_PATH, executeGate } from '../../lib/runner.ts';
import fs from 'node:fs';
import path from 'node:path';

const DEFAULT_OUT = path.join(
  process.cwd(),
  'core/local/artifacts/release_candidate_dress_rehearsal_current.json',
);
const CLOSURE_POLICY_PATH = path.join(
  process.cwd(),
  'client/runtime/config/production_readiness_closure_policy.json',
);
const REQUIRED_72H_BOUNDEDNESS_ARTIFACTS = [
  'runtime_boundedness_72h_evidence_current.json',
  'runtime_boundedness_profiles_current.json',
];
const PREBUNDLE_BLOCKING_STEP_GATE_IDS = new Set<string>(['release_policy_gate']);

const DEFAULT_SEQUENCE = [
  'dr:gameday',
  'dr:gameday',
  'dr:gameday',
  'dr:gameday',
  'dr:gameday:gate',
  'chaos:continuous:gate',
  'state:kernel:replay',
  'ops:runtime-proof:verify',
  'release_policy_gate',
  'ops:windows-installer:contract:guard',
  'ops:legacy-runner:release-guard',
  'ops:production-topology:gate',
  'audit:shell-layer-boundary',
  'ops:stateful-upgrade-rollback:gate',
  'ops:assimilation:v1:support:guard',
  'ops:orchestration:hidden-state:guard',
  'ops:release-blockers:gate',
  'ops:release-hardening-window:guard',
  'ops:support-bundle:export',
  'ops:layer2:parity:guard',
  'ops:layer2:receipt:replay',
  'ops:trusted-core:report',
  'ops:srs:todo-section:guard',
  'ops:ipc-bridge:soak',
  'ops:release:scorecard:gate',
  'ops:production-closure:gate',
  'ops:release:proof-pack',
];

function readRehearsalArgs(argv: string[]) {
  const parsed = parseStrictOutArgs(argv, { out: DEFAULT_OUT, strict: false });
  const activateHardening = parseBool(readFlag(argv, 'activate-hardening'), true);
  const stageRaw = clean(readFlag(argv, 'stage') || 'prebundle', 40).toLowerCase();
  return {
    strict: parsed.strict,
    out: parsed.out || DEFAULT_OUT,
    activateHardening,
    stage: stageRaw === 'final' ? 'final' : 'prebundle',
  };
}

function clean(value: unknown, max = 240): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function parseJsonPayload(raw: string): any {
  const whole = String(raw || '').trim();
  if (!whole) return null;
  try {
    return JSON.parse(whole);
  } catch {
    return null;
  }
}

function hasRequiredArtifacts(paths: unknown, requiredFiles: string[]): { ok: boolean; present: string[] } {
  const allPaths = Array.isArray(paths) ? paths.map((row) => clean(row, 320)).filter(Boolean) : [];
  const present = requiredFiles.filter((required) => allPaths.some((candidate) => candidate.endsWith(required)));
  return {
    ok: present.length === requiredFiles.length,
    present,
  };
}

function readRequiredStepIds(): string[] {
  try {
    const policy = JSON.parse(fs.readFileSync(CLOSURE_POLICY_PATH, 'utf8'));
    const configured = policy?.release_candidate_rehearsal?.required_step_gate_ids;
    return Array.isArray(configured) ? configured.map((row: unknown) => clean(row, 160)).filter(Boolean) : [];
  } catch {
    return [];
  }
}

function buildReport(argv: string[] = process.argv.slice(2)) {
  const args = readRehearsalArgs(argv);
  const finalStage = args.stage === 'final';
  const stageMode = finalStage ? 'strict_final' : 'prebundle_mixed';
  const requiredStepGateIds = readRequiredStepIds();
  const previousHardeningValue = process.env.INFRING_RELEASE_HARDENING_WINDOW;
  const previousRcActiveValue = process.env.INFRING_RELEASE_RC_REHEARSAL_ACTIVE;
  if (args.activateHardening) process.env.INFRING_RELEASE_HARDENING_WINDOW = '1';
  process.env.INFRING_RELEASE_RC_REHEARSAL_ACTIVE = '1';
  try {
    const steps = DEFAULT_SEQUENCE.map((gateId, index) => {
      const report = executeGate(gateId, {
        registryPath: DEFAULT_GATE_REGISTRY_PATH,
        strict: true,
      });
      const payload = parseJsonPayload(report.stdout);
      return {
        order: index + 1,
        gate_id: gateId,
        ok: report.ok,
        duration_ms: report.duration_ms,
        exit_code: report.summary.exit_code,
        artifact_paths: report.artifact_paths,
        failure: report.failures[0]?.detail || '',
        payload_type: clean(payload?.type || '', 120),
        gate_state: clean(payload?.gate_state || '', 120),
        failed_ids: Array.isArray(payload?.failed_ids) ? payload.failed_ids : [],
        degraded_flags: Array.isArray(payload?.degraded_flags) ? payload.degraded_flags : [],
        payload_summary: payload?.summary || null,
      };
    });
    const failed = steps.filter((row) => !row.ok);
    const blockingFailures = finalStage
      ? failed
      : failed.filter((row) => PREBUNDLE_BLOCKING_STEP_GATE_IDS.has(row.gate_id));
    const nonBlockingFailures = failed.filter(
      (row) => !blockingFailures.some((blocking) => blocking.gate_id === row.gate_id),
    );
    const stepGateIds = steps.map((row) => clean(row.gate_id, 160)).filter(Boolean);
    const stepGateIdSet = new Set(stepGateIds);
    const stepGateIdsUnique = stepGateIds.length === stepGateIdSet.size;
    const stepOrderContiguous = steps.every((row, index) => row.order === index + 1);
    const failureGateIds = failed.map((row) => clean(row.gate_id, 160)).filter(Boolean);
    const blockingFailureGateIds = blockingFailures
      .map((row) => clean(row.gate_id, 160))
      .filter(Boolean);
    const nonBlockingFailureGateIds = nonBlockingFailures
      .map((row) => clean(row.gate_id, 160))
      .filter(Boolean);
    const failureGateIdSet = new Set(failureGateIds);
    const blockingFailureGateIdSet = new Set(blockingFailureGateIds);
    const nonBlockingFailureGateIdSet = new Set(nonBlockingFailureGateIds);
    const failureGateIdsUnique = failureGateIds.length === failureGateIdSet.size;
    const blockingFailureGateIdsUnique =
      blockingFailureGateIds.length === blockingFailureGateIdSet.size;
    const nonBlockingFailureGateIdsUnique =
      nonBlockingFailureGateIds.length === nonBlockingFailureGateIdSet.size;
    const blockingNonBlockingDisjoint = blockingFailureGateIds.every(
      (gateId) => !nonBlockingFailureGateIdSet.has(gateId),
    );
    const partitionCountMatches =
      failed.length === blockingFailures.length + nonBlockingFailures.length;
    const passedGateIds = new Set(steps.filter((row) => row.ok).map((row) => row.gate_id));
    const requiredStepGateIdSet = new Set(requiredStepGateIds);
    const requiredStepGateIdsUnique = requiredStepGateIds.length === requiredStepGateIdSet.size;
    const requiredStepMissingGateIds = finalStage
      ? requiredStepGateIds.filter((gateId) => !passedGateIds.has(gateId))
      : [];
    const requiredStepMissingGateIdSet = new Set(requiredStepMissingGateIds);
    const requiredStepMissingGateIdsUnique =
      requiredStepMissingGateIds.length === requiredStepMissingGateIdSet.size;
    const requiredStepMissingCountMatches = finalStage
      ? requiredStepMissingGateIds.length ===
        Math.max(0, requiredStepGateIds.length - Array.from(passedGateIds).filter((gateId) => requiredStepGateIdSet.has(gateId)).length)
      : requiredStepMissingGateIds.length === 0;
    const emittedArtifactPaths = steps.flatMap((row) =>
      Array.isArray(row.artifact_paths) ? row.artifact_paths : [],
    );
    const emittedArtifactPathTokens = emittedArtifactPaths
      .map((row) => clean(row, 320))
      .filter(Boolean);
    const emittedArtifactPathSet = new Set(emittedArtifactPathTokens);
    const requiredStepsSatisfied = finalStage
      ? requiredStepGateIds.length === 0 ||
        requiredStepGateIds.every((gateId) => passedGateIds.has(gateId))
      : true;
    const recoveryStep = steps.find((row) => row.gate_id === 'dr:gameday:gate');
    const chaosStep = steps.find((row) => row.gate_id === 'chaos:continuous:gate');
    const replayStep = steps.find((row) => row.gate_id === 'state:kernel:replay');
    const runtimeProofStep = steps.find((row) => row.gate_id === 'ops:runtime-proof:verify');
    const topologyStep = steps.find((row) => row.gate_id === 'ops:production-topology:gate');
    const shellBoundaryStep = steps.find((row) => row.gate_id === 'audit:shell-layer-boundary');
    const hiddenStateStep = steps.find((row) => row.gate_id === 'ops:orchestration:hidden-state:guard');
    const layer2ParityStep = steps.find((row) => row.gate_id === 'ops:layer2:parity:guard');
    const layer2ReplayStep = steps.find((row) => row.gate_id === 'ops:layer2:receipt:replay');
    const trustedCoreStep = steps.find((row) => row.gate_id === 'ops:trusted-core:report');
    const proofPackStep = steps.find((row) => row.gate_id === 'ops:release:proof-pack');
    const windowsInstallerContractStep = steps.find(
      (row) => row.gate_id === 'ops:windows-installer:contract:guard',
    );
    const boundednessArtifacts = hasRequiredArtifacts(
      runtimeProofStep?.artifact_paths,
      REQUIRED_72H_BOUNDEDNESS_ARTIFACTS,
    );
    const boundednessRequired = finalStage;
    const boundednessSatisfied = boundednessRequired ? boundednessArtifacts.ok : true;
    return {
      ok: blockingFailures.length === 0 && requiredStepsSatisfied && boundednessSatisfied,
      type: 'release_candidate_dress_rehearsal',
      generated_at: new Date().toISOString(),
      strict: args.strict,
      inputs: {
        stage: args.stage,
        activate_hardening_window: args.activateHardening,
        registry_path: DEFAULT_GATE_REGISTRY_PATH,
        required_step_gate_ids: requiredStepGateIds,
      },
      summary: {
        step_count: steps.length,
        failed_count: failed.length,
        blocking_failed_count: blockingFailures.length,
        non_blocking_failed_count: nonBlockingFailures.length,
        stage_mode: stageMode,
        required_step_count: requiredStepGateIds.length,
        required_steps_enforced: finalStage,
        required_steps_satisfied: requiredStepsSatisfied,
        required_step_ids_unique: requiredStepGateIdsUnique,
        required_steps_missing_count: requiredStepMissingGateIds.length,
        required_steps_missing_ids_unique: requiredStepMissingGateIdsUnique,
        required_steps_missing_count_matches: requiredStepMissingCountMatches,
        boundedness_required_artifacts_enforced: boundednessRequired,
        boundedness_required_artifacts_present: boundednessArtifacts.ok,
        step_gate_ids_unique: stepGateIdsUnique,
        step_order_contiguous: stepOrderContiguous,
        failure_gate_ids_unique: failureGateIdsUnique,
        blocking_failure_gate_ids_unique: blockingFailureGateIdsUnique,
        non_blocking_failure_gate_ids_unique: nonBlockingFailureGateIdsUnique,
        blocking_non_blocking_disjoint: blockingNonBlockingDisjoint,
        failure_partition_count_matches: partitionCountMatches,
        emitted_artifact_path_count: emittedArtifactPathTokens.length,
        emitted_artifact_path_unique_count: emittedArtifactPathSet.size,
        candidate_ready:
          blockingFailures.length === 0 && requiredStepsSatisfied && boundednessSatisfied,
      },
      failures: failed,
      blocking_failures: blockingFailures,
      non_blocking_failures: nonBlockingFailures,
      failure_gate_ids: failureGateIds,
      blocking_failure_gate_ids: blockingFailureGateIds,
      non_blocking_failure_gate_ids: nonBlockingFailureGateIds,
      required_steps_missing_gate_ids: requiredStepMissingGateIds,
      artifact_paths: emittedArtifactPaths,
      recovery_rehearsal: {
        gate_state: clean(recoveryStep?.gate_state || '', 120),
        ok: recoveryStep?.ok === true,
      },
      chaos: {
        ok: chaosStep?.ok === true,
        payload_type: clean(chaosStep?.payload_type || '', 120),
        artifact_paths: chaosStep?.artifact_paths || [],
      },
      replay: {
        ok: replayStep?.ok === true,
        payload_type: clean(replayStep?.payload_type || '', 120),
        artifact_paths: replayStep?.artifact_paths || [],
      },
      runtime_proof: {
        ok: runtimeProofStep?.ok === true,
        payload_type: clean(runtimeProofStep?.payload_type || '', 120),
        artifact_paths: runtimeProofStep?.artifact_paths || [],
      },
      boundedness_72h: {
        required_artifacts: REQUIRED_72H_BOUNDEDNESS_ARTIFACTS,
        required_artifacts_present: boundednessArtifacts.ok,
        present_artifacts: boundednessArtifacts.present,
      },
      topology: {
        ok: topologyStep?.ok === true,
        degraded_flags: topologyStep?.degraded_flags || [],
      },
      shell_boundary: {
        ok: shellBoundaryStep?.ok === true,
        failed_ids: shellBoundaryStep?.failed_ids || [],
      },
      hidden_state: {
        ok: hiddenStateStep?.ok === true,
        failure: clean(hiddenStateStep?.failure || '', 200),
      },
      layer2_parity: {
        ok: layer2ParityStep?.ok === true,
        payload_type: clean(layer2ParityStep?.payload_type || '', 120),
        artifact_paths: layer2ParityStep?.artifact_paths || [],
      },
      layer2_receipt_replay: {
        ok: layer2ReplayStep?.ok === true,
        payload_type: clean(layer2ReplayStep?.payload_type || '', 120),
        artifact_paths: layer2ReplayStep?.artifact_paths || [],
      },
      trusted_core: {
        ok: trustedCoreStep?.ok === true,
        payload_type: clean(trustedCoreStep?.payload_type || '', 120),
        artifact_paths: trustedCoreStep?.artifact_paths || [],
      },
      windows_installer_contract: {
        ok: windowsInstallerContractStep?.ok === true,
        payload_type: clean(windowsInstallerContractStep?.payload_type || '', 120),
        artifact_paths: windowsInstallerContractStep?.artifact_paths || [],
      },
      proof_pack: {
        ok: proofPackStep?.ok === true,
        payload_type: clean(proofPackStep?.payload_type || '', 120),
        artifact_paths: proofPackStep?.artifact_paths || [],
      },
      steps,
    };
  } finally {
    if (args.activateHardening) {
      if (previousHardeningValue == null) delete process.env.INFRING_RELEASE_HARDENING_WINDOW;
      else process.env.INFRING_RELEASE_HARDENING_WINDOW = previousHardeningValue;
    }
    if (previousRcActiveValue == null) delete process.env.INFRING_RELEASE_RC_REHEARSAL_ACTIVE;
    else process.env.INFRING_RELEASE_RC_REHEARSAL_ACTIVE = previousRcActiveValue;
  }
}

function run(argv: string[] = process.argv.slice(2)) {
  const args = readRehearsalArgs(argv);
  const report = buildReport(argv);
  return emitStructuredResult(report, {
    outPath: args.out,
    strict: args.strict,
    ok: report.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  buildReport,
  run,
};
