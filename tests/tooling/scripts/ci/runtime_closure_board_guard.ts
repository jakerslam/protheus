#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

type Bucket = {
  id: string;
  label: string;
  owner: string;
  status: string;
  validation_gates: string[];
  evidence_artifacts: string[];
};

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/runtime_closure_board_guard_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    boardPath: cleanText(
      readFlag(argv, 'board') || 'client/runtime/config/runtime_closure_board.json',
      400,
    ),
    gateRegistryPath: cleanText(
      readFlag(argv, 'gate-registry') || 'tests/tooling/config/tooling_gate_registry.json',
      400,
    ),
    verifyProfilesPath: cleanText(
      readFlag(argv, 'verify-profiles') || 'tests/tooling/config/verify_profiles.json',
      400,
    ),
    markdownPath: cleanText(
      readFlag(argv, 'out-markdown') || 'local/workspace/reports/RUNTIME_CLOSURE_BOARD_GUARD_CURRENT.md',
      400,
    ),
  };
}

function readJsonBestEffort(filePath: string): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function toStringList(raw: unknown, maxLen = 200): string[] {
  if (!Array.isArray(raw)) return [];
  const out: string[] = [];
  for (const value of raw) {
    const cleaned = cleanText(value || '', maxLen);
    if (!cleaned) continue;
    out.push(cleaned);
  }
  return out;
}

function duplicateTokens(values: string[]): string[] {
  const seen = new Set<string>();
  const duplicates = new Set<string>();
  for (const value of values) {
    if (!value) continue;
    if (seen.has(value)) duplicates.add(value);
    seen.add(value);
  }
  return Array.from(duplicates).sort();
}

function isCanonicalRelativePath(value: string, requiredPrefix = '', requiredSuffix = ''): boolean {
  const normalized = cleanText(value || '', 500);
  if (!normalized) return false;
  if (path.isAbsolute(normalized)) return false;
  if (normalized.includes('\\')) return false;
  if (normalized.includes('..')) return false;
  if (normalized.includes('//')) return false;
  if (normalized.endsWith('/')) return false;
  if (/\s/.test(normalized)) return false;
  if (requiredPrefix && !normalized.startsWith(requiredPrefix)) return false;
  if (requiredSuffix && !normalized.endsWith(requiredSuffix)) return false;
  return true;
}

function renderMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Runtime Closure Board Guard');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(payload?.generated_at || '', 80)}`);
  lines.push(`- revision: ${cleanText(payload?.revision || '', 120)}`);
  lines.push(`- pass: ${payload?.ok === true ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- bucket_count: ${Number(payload?.summary?.bucket_count || 0)}`);
  lines.push(`- required_bucket_count: ${Number(payload?.summary?.required_bucket_count || 0)}`);
  lines.push(`- missing_required_bucket_count: ${Number(payload?.summary?.missing_required_bucket_count || 0)}`);
  lines.push(`- invalid_gate_ref_count: ${Number(payload?.summary?.invalid_gate_ref_count || 0)}`);
  lines.push(`- failure_count: ${Number(payload?.summary?.failure_count || 0)}`);
  lines.push('');
  lines.push('## Buckets');
  for (const bucket of Array.isArray(payload?.buckets) ? payload.buckets : []) {
    lines.push(
      `- ${cleanText(bucket?.id || 'unknown', 80)}: status=${cleanText(
        bucket?.status || '',
        40,
      )} gates=${Number(bucket?.validation_gate_count || 0)} artifacts=${Number(
        bucket?.evidence_artifact_count || 0,
      )} missing_gate_refs=${Number(bucket?.missing_gate_refs_count || 0)}`,
    );
  }
  const failures = Array.isArray(payload?.failures) ? payload.failures : [];
  if (failures.length > 0) {
    lines.push('');
    lines.push('## Failures');
    for (const failure of failures) {
      lines.push(
        `- ${cleanText(failure?.id || 'unknown', 120)}: ${cleanText(failure?.detail || '', 240)}`,
      );
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function writeMarkdown(filePath: string, body: string): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, body, 'utf8');
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const expectedSchemaId = 'runtime_closure_board';
  const expectedSchemaVersion = 1;
  const expectedTopLevelKeys = new Set(['schema_id', 'schema_version', 'updated_at', 'buckets']);
  const maxBoardAgeMs = 14 * 24 * 60 * 60 * 1000;
  const allowedFutureSkewMs = 5 * 60 * 1000;
  const expectedBucketOrder = [
    'layer2_parity',
    'production_gateways',
    'boundedness',
    'dashboard_truth',
    'auto_heal_backpressure',
  ];
  const expectedBucketLabels = new Map<string, string>([
    ['layer2_parity', 'Layer 2 parity'],
    ['production_gateways', 'Production gateways'],
    ['boundedness', 'Boundedness'],
    ['dashboard_truth', 'Dashboard truth'],
    ['auto_heal_backpressure', 'Auto-heal and backpressure'],
  ]);
  const expectedBucketOwners = new Map<string, string>([
    ['layer2_parity', 'kernel-runtime'],
    ['production_gateways', 'gateway-runtime'],
    ['boundedness', 'runtime-proof'],
    ['dashboard_truth', 'shell-authority'],
    ['auto_heal_backpressure', 'runtime-recovery'],
  ]);
  const expectedBucketStatuses = new Map<string, string>([
    ['layer2_parity', 'active'],
    ['production_gateways', 'active'],
    ['boundedness', 'active'],
    ['dashboard_truth', 'active'],
    ['auto_heal_backpressure', 'active'],
  ]);
  const expectedBucketValidationGates = new Map<string, string[]>([
    ['layer2_parity', ['ops:layer2:parity:guard', 'ops:layer2:receipt:replay']],
    ['production_gateways', ['ops:gateway-runtime-chaos:gate']],
    ['boundedness', ['ops:runtime-proof:verify', 'ops:boundedness:release-gate']],
    ['dashboard_truth', ['ops:dashboard:surface:guard', 'ops:shell:truth-leak:guard']],
    ['auto_heal_backpressure', ['ops:queue-backpressure:policy:gate', 'ops:runtime-proof:verify']],
  ]);
  const expectedBucketEvidenceArtifacts = new Map<string, string[]>([
    [
      'layer2_parity',
      [
        'core/local/artifacts/layer2_lane_parity_guard_current.json',
        'core/local/artifacts/layer2_receipt_replay_current.json',
      ],
    ],
    [
      'production_gateways',
      [
        'core/local/artifacts/gateway_runtime_chaos_gate_current.json',
        'core/local/artifacts/gateway_support_levels_current.json',
      ],
    ],
    [
      'boundedness',
      [
        'core/local/artifacts/runtime_boundedness_72h_evidence_current.json',
        'core/local/artifacts/runtime_boundedness_release_gate_current.json',
      ],
    ],
    [
      'dashboard_truth',
      [
        'core/local/artifacts/dashboard_surface_authority_guard_current.json',
        'core/local/artifacts/shell_truth_leak_guard_current.json',
      ],
    ],
    [
      'auto_heal_backpressure',
      [
        'core/local/artifacts/queue_backpressure_policy_gate_current.json',
        'core/local/artifacts/runtime_proof_verify_current.json',
      ],
    ],
  ]);
  const globallyShareableValidationGates = new Set<string>(['ops:runtime-proof:verify']);
  const gateIdPattern = /^ops:[a-z0-9:-]+$/;
  const requiredBucketIds = new Set([
    'layer2_parity',
    'production_gateways',
    'boundedness',
    'dashboard_truth',
    'auto_heal_backpressure',
  ]);
  const runtimeCriticalBucketIds = new Set([
    'layer2_parity',
    'production_gateways',
    'boundedness',
    'auto_heal_backpressure',
  ]);
  const allowedStatuses = new Set(['active', 'blocked', 'degraded', 'done']);
  const expectedBucketKeys = new Set([
    'id',
    'label',
    'owner',
    'status',
    'validation_gates',
    'evidence_artifacts',
  ]);
  const outPathCanonical = isCanonicalRelativePath(
    args.outPath,
    'core/local/artifacts/',
    'runtime_closure_board_guard_current.json',
  );
  const boardPathCanonical = isCanonicalRelativePath(
    args.boardPath,
    'client/runtime/config/',
    'runtime_closure_board.json',
  );
  const gateRegistryPathCanonical = isCanonicalRelativePath(
    args.gateRegistryPath,
    'tests/tooling/config/',
    'tooling_gate_registry.json',
  );
  const verifyProfilesPathCanonical = isCanonicalRelativePath(
    args.verifyProfilesPath,
    'tests/tooling/config/',
    'verify_profiles.json',
  );
  const markdownPathCanonical = isCanonicalRelativePath(
    args.markdownPath,
    'local/workspace/reports/',
    'RUNTIME_CLOSURE_BOARD_GUARD_CURRENT.md',
  );
  const inputPathsDistinct =
    new Set([args.boardPath, args.gateRegistryPath, args.verifyProfilesPath, args.outPath, args.markdownPath])
      .size === 5;
  const boardPayload = readJsonBestEffort(path.resolve(root, args.boardPath));
  const gateRegistry = readJsonBestEffort(path.resolve(root, args.gateRegistryPath));
  const verifyProfiles = readJsonBestEffort(path.resolve(root, args.verifyProfilesPath));
  const knownGateIdsList = Object.keys(gateRegistry?.gates || {})
    .map((value) => cleanText(value, 160))
    .filter(Boolean);
  const knownGateIds = new Set<string>(knownGateIdsList);
  const releaseProfileGateIdsList = Array.isArray(verifyProfiles?.profiles?.release?.gate_ids)
    ? verifyProfiles.profiles.release.gate_ids.map((value: unknown) => cleanText(value, 160)).filter(Boolean)
    : [];
  const releaseProfileGateIds = new Set<string>(releaseProfileGateIdsList);
  const runtimeProofProfileGateIdsList = Array.isArray(verifyProfiles?.profiles?.['runtime-proof']?.gate_ids)
    ? verifyProfiles.profiles['runtime-proof'].gate_ids
        .map((value: unknown) => cleanText(value, 160))
        .filter(Boolean)
    : [];
  const runtimeProofProfileGateIds = new Set<string>(runtimeProofProfileGateIdsList);
  const expectedClosureGateIds = Array.from(
    new Set(expectedBucketOrder.flatMap((bucketId) => expectedBucketValidationGates.get(bucketId) || [])),
  ).sort();
  const expectedRuntimeCriticalGateIds = Array.from(
    new Set(Array.from(runtimeCriticalBucketIds).flatMap((bucketId) => expectedBucketValidationGates.get(bucketId) || [])),
  ).sort();
  const failures: Array<{ id: string; detail: string }> = [];

  if (!outPathCanonical) {
    failures.push({
      id: 'runtime_closure_guard_out_path_contract_invalid',
      detail: args.outPath,
    });
  }
  if (!boardPathCanonical) {
    failures.push({
      id: 'runtime_closure_guard_board_path_contract_invalid',
      detail: args.boardPath,
    });
  }
  if (!gateRegistryPathCanonical) {
    failures.push({
      id: 'runtime_closure_guard_gate_registry_path_contract_invalid',
      detail: args.gateRegistryPath,
    });
  }
  if (!verifyProfilesPathCanonical) {
    failures.push({
      id: 'runtime_closure_guard_verify_profiles_path_contract_invalid',
      detail: args.verifyProfilesPath,
    });
  }
  if (!markdownPathCanonical) {
    failures.push({
      id: 'runtime_closure_guard_markdown_path_contract_invalid',
      detail: args.markdownPath,
    });
  }
  if (!inputPathsDistinct) {
    failures.push({
      id: 'runtime_closure_guard_input_paths_must_be_distinct',
      detail: `${args.boardPath};${args.gateRegistryPath};${args.verifyProfilesPath};${args.outPath};${args.markdownPath}`,
    });
  }

  if (!gateRegistry || typeof gateRegistry !== 'object' || Array.isArray(gateRegistry)) {
    failures.push({
      id: 'runtime_closure_guard_gate_registry_payload_invalid',
      detail: args.gateRegistryPath,
    });
  } else {
    if (!gateRegistry.gates || typeof gateRegistry.gates !== 'object' || Array.isArray(gateRegistry.gates)) {
      failures.push({
        id: 'runtime_closure_guard_gate_registry_gates_object_missing',
        detail: args.gateRegistryPath,
      });
    }
    if (knownGateIds.size === 0) {
      failures.push({
        id: 'runtime_closure_guard_gate_registry_empty',
        detail: args.gateRegistryPath,
      });
    }
  }

  if (!verifyProfiles || typeof verifyProfiles !== 'object' || Array.isArray(verifyProfiles)) {
    failures.push({
      id: 'runtime_closure_guard_verify_profiles_payload_invalid',
      detail: args.verifyProfilesPath,
    });
  } else {
    if (!Array.isArray(verifyProfiles?.profiles?.release?.gate_ids)) {
      failures.push({
        id: 'runtime_closure_guard_verify_profiles_release_gate_ids_missing',
        detail: args.verifyProfilesPath,
      });
    }
    if (!Array.isArray(verifyProfiles?.profiles?.['runtime-proof']?.gate_ids)) {
      failures.push({
        id: 'runtime_closure_guard_verify_profiles_runtime_proof_gate_ids_missing',
        detail: args.verifyProfilesPath,
      });
    }
  }

  if (releaseProfileGateIdsList.length === 0) {
    failures.push({
      id: 'runtime_closure_guard_release_profile_gate_ids_empty',
      detail: args.verifyProfilesPath,
    });
  }
  const duplicateReleaseProfileGateIds = duplicateTokens(releaseProfileGateIdsList);
  if (duplicateReleaseProfileGateIds.length > 0) {
    failures.push({
      id: 'runtime_closure_guard_release_profile_gate_ids_duplicate',
      detail: duplicateReleaseProfileGateIds.join(','),
    });
  }
  const noncanonicalReleaseProfileGateIds = releaseProfileGateIdsList.filter(
    (gateId) => !gateIdPattern.test(gateId),
  );
  if (noncanonicalReleaseProfileGateIds.length > 0) {
    failures.push({
      id: 'runtime_closure_guard_release_profile_gate_ids_noncanonical',
      detail: noncanonicalReleaseProfileGateIds.join(','),
    });
  }
  if (releaseProfileGateIdsList.join('|') !== [...releaseProfileGateIdsList].sort().join('|')) {
    failures.push({
      id: 'runtime_closure_guard_release_profile_gate_ids_unsorted',
      detail: releaseProfileGateIdsList.join(','),
    });
  }

  if (runtimeProofProfileGateIdsList.length === 0) {
    failures.push({
      id: 'runtime_closure_guard_runtime_proof_profile_gate_ids_empty',
      detail: args.verifyProfilesPath,
    });
  }
  const duplicateRuntimeProofProfileGateIds = duplicateTokens(runtimeProofProfileGateIdsList);
  if (duplicateRuntimeProofProfileGateIds.length > 0) {
    failures.push({
      id: 'runtime_closure_guard_runtime_proof_profile_gate_ids_duplicate',
      detail: duplicateRuntimeProofProfileGateIds.join(','),
    });
  }
  const noncanonicalRuntimeProofProfileGateIds = runtimeProofProfileGateIdsList.filter(
    (gateId) => !gateIdPattern.test(gateId),
  );
  if (noncanonicalRuntimeProofProfileGateIds.length > 0) {
    failures.push({
      id: 'runtime_closure_guard_runtime_proof_profile_gate_ids_noncanonical',
      detail: noncanonicalRuntimeProofProfileGateIds.join(','),
    });
  }
  if (
    runtimeProofProfileGateIdsList.join('|') !== [...runtimeProofProfileGateIdsList].sort().join('|')
  ) {
    failures.push({
      id: 'runtime_closure_guard_runtime_proof_profile_gate_ids_unsorted',
      detail: runtimeProofProfileGateIdsList.join(','),
    });
  }

  const unknownReleaseProfileGateIds = releaseProfileGateIdsList.filter(
    (gateId) => !knownGateIds.has(gateId),
  );
  if (unknownReleaseProfileGateIds.length > 0) {
    failures.push({
      id: 'runtime_closure_guard_release_profile_gate_ids_unknown_to_registry',
      detail: unknownReleaseProfileGateIds.join(','),
    });
  }
  const unknownRuntimeProofProfileGateIds = runtimeProofProfileGateIdsList.filter(
    (gateId) => !knownGateIds.has(gateId),
  );
  if (unknownRuntimeProofProfileGateIds.length > 0) {
    failures.push({
      id: 'runtime_closure_guard_runtime_proof_profile_gate_ids_unknown_to_registry',
      detail: unknownRuntimeProofProfileGateIds.join(','),
    });
  }

  const noncanonicalKnownGateIds = knownGateIdsList.filter((gateId) => !gateIdPattern.test(gateId));
  if (noncanonicalKnownGateIds.length > 0) {
    failures.push({
      id: 'runtime_closure_guard_gate_registry_gate_ids_noncanonical',
      detail: noncanonicalKnownGateIds.join(','),
    });
  }
  if (knownGateIdsList.join('|') !== [...knownGateIdsList].sort().join('|')) {
    failures.push({
      id: 'runtime_closure_guard_gate_registry_gate_ids_unsorted',
      detail: knownGateIdsList.join(','),
    });
  }

  const expectedClosureGateIdsMissingInRegistry = expectedClosureGateIds.filter(
    (gateId) => !knownGateIds.has(gateId),
  );
  if (expectedClosureGateIdsMissingInRegistry.length > 0) {
    failures.push({
      id: 'runtime_closure_guard_expected_closure_gates_missing_in_registry',
      detail: expectedClosureGateIdsMissingInRegistry.join(','),
    });
  }
  const expectedClosureGateIdsMissingInReleaseProfile = expectedClosureGateIds.filter(
    (gateId) => !releaseProfileGateIds.has(gateId),
  );
  if (expectedClosureGateIdsMissingInReleaseProfile.length > 0) {
    failures.push({
      id: 'runtime_closure_guard_expected_closure_gates_missing_in_release_profile',
      detail: expectedClosureGateIdsMissingInReleaseProfile.join(','),
    });
  }
  const expectedRuntimeCriticalGateIdsMissingInRuntimeProofProfile = expectedRuntimeCriticalGateIds.filter(
    (gateId) => !runtimeProofProfileGateIds.has(gateId),
  );
  if (expectedRuntimeCriticalGateIdsMissingInRuntimeProofProfile.length > 0) {
    failures.push({
      id: 'runtime_closure_guard_expected_runtime_critical_gates_missing_in_runtime_proof_profile',
      detail: expectedRuntimeCriticalGateIdsMissingInRuntimeProofProfile.join(','),
    });
  }

  const releaseRuntimeProofOverlapCount = releaseProfileGateIdsList.filter((gateId) =>
    runtimeProofProfileGateIds.has(gateId),
  ).length;
  if (releaseRuntimeProofOverlapCount === 0) {
    failures.push({
      id: 'runtime_closure_guard_release_runtime_proof_overlap_missing',
      detail: 'release/runtime-proof gate overlap=0',
    });
  }
  const runtimeProofGateIdsOutsideRelease = runtimeProofProfileGateIdsList.filter(
    (gateId) => !releaseProfileGateIds.has(gateId),
  );
  if (runtimeProofGateIdsOutsideRelease.length > 0) {
    failures.push({
      id: 'runtime_closure_guard_runtime_proof_profile_not_subset_of_release_profile',
      detail: runtimeProofGateIdsOutsideRelease.join(','),
    });
  }
  if (!releaseProfileGateIds.has('ops:runtime-proof:verify')) {
    failures.push({
      id: 'runtime_closure_guard_release_profile_runtime_proof_verify_missing',
      detail: 'ops:runtime-proof:verify',
    });
  }
  if (!runtimeProofProfileGateIds.has('ops:runtime-proof:verify')) {
    failures.push({
      id: 'runtime_closure_guard_runtime_proof_profile_runtime_proof_verify_missing',
      detail: 'ops:runtime-proof:verify',
    });
  }
  if (expectedClosureGateIds.length === 0) {
    failures.push({
      id: 'runtime_closure_guard_expected_closure_gate_set_empty',
      detail: 'expected closure gate set is empty',
    });
  }

  if (!boardPayload) {
    failures.push({
      id: 'runtime_closure_board_missing',
      detail: args.boardPath,
    });
  } else {
    if (typeof boardPayload !== 'object' || Array.isArray(boardPayload)) {
      failures.push({
        id: 'runtime_closure_board_payload_object_invalid',
        detail: args.boardPath,
      });
    }
    const schemaId = cleanText(boardPayload?.schema_id || '', 80);
    const schemaVersion = Number(boardPayload?.schema_version || 0);
    const updatedAt = cleanText(boardPayload?.updated_at || '', 80);
    const topLevelKeys = Object.keys(boardPayload || {});
    const missingTopLevelKeys = Array.from(expectedTopLevelKeys).filter(
      (key) => !Object.prototype.hasOwnProperty.call(boardPayload, key),
    );
    if (missingTopLevelKeys.length > 0) {
      failures.push({
        id: 'runtime_closure_board_schema_keys_missing',
        detail: missingTopLevelKeys.join(','),
      });
    }
    const unknownTopLevelKeys = topLevelKeys.filter((key) => !expectedTopLevelKeys.has(key));
    if (unknownTopLevelKeys.length > 0) {
      failures.push({
        id: 'runtime_closure_board_schema_keys_unknown',
        detail: unknownTopLevelKeys.join(','),
      });
    }
    if (schemaId !== expectedSchemaId) {
      failures.push({
        id: 'runtime_closure_board_schema_id_invalid',
        detail: schemaId || 'missing',
      });
    }
    if (schemaVersion !== expectedSchemaVersion) {
      failures.push({
        id: 'runtime_closure_board_schema_version_invalid',
        detail: Number.isFinite(schemaVersion) ? String(schemaVersion) : 'missing',
      });
    }
    if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/.test(updatedAt)) {
      failures.push({
        id: 'runtime_closure_board_updated_at_invalid',
        detail: updatedAt || 'missing',
      });
    } else {
      const updatedAtMs = Date.parse(updatedAt);
      if (!Number.isFinite(updatedAtMs)) {
        failures.push({
          id: 'runtime_closure_board_updated_at_unparseable',
          detail: updatedAt,
        });
      } else {
        const nowMs = Date.now();
        if (updatedAtMs > nowMs + allowedFutureSkewMs) {
          failures.push({
            id: 'runtime_closure_board_updated_at_future',
            detail: updatedAt,
          });
        }
        if (nowMs - updatedAtMs > maxBoardAgeMs) {
          failures.push({
            id: 'runtime_closure_board_updated_at_stale',
            detail: updatedAt,
          });
        }
      }
    }
  }

  const bucketRowsRaw = Array.isArray(boardPayload?.buckets) ? boardPayload.buckets : [];
  if (!Array.isArray(boardPayload?.buckets)) {
    failures.push({
      id: 'runtime_closure_board_buckets_not_array',
      detail: args.boardPath,
    });
  }
  if (bucketRowsRaw.length !== expectedBucketOrder.length) {
    failures.push({
      id: 'runtime_closure_bucket_raw_count_noncanonical',
      detail: `${bucketRowsRaw.length};expected=${expectedBucketOrder.length}`,
    });
  }
  const buckets: Bucket[] = bucketRowsRaw.map((row: any) => ({
    id: cleanText(row?.id || '', 80),
    label: cleanText(row?.label || '', 120),
    owner: cleanText(row?.owner || '', 80),
    status: cleanText(row?.status || '', 40),
    validation_gates: toStringList(row?.validation_gates, 120),
    evidence_artifacts: toStringList(row?.evidence_artifacts, 260),
  }));

  const byId = new Map<string, Bucket>();
  for (const bucket of buckets) {
    if (!bucket.id) {
      failures.push({
        id: 'runtime_closure_bucket_id_missing',
        detail: bucket.label || 'unknown',
      });
      continue;
    }
    if (byId.has(bucket.id)) {
      failures.push({
        id: 'runtime_closure_bucket_duplicate',
        detail: bucket.id,
      });
      continue;
    }
    byId.set(bucket.id, bucket);
  }

  const unknownBucketIds = Array.from(byId.keys()).filter((bucketId) => !requiredBucketIds.has(bucketId));
  if (unknownBucketIds.length > 0) {
    failures.push({
      id: 'runtime_closure_bucket_unknown',
      detail: unknownBucketIds.join(','),
    });
  }

  if (byId.size !== requiredBucketIds.size) {
    failures.push({
      id: 'runtime_closure_bucket_count_mismatch',
      detail: `${byId.size}/${requiredBucketIds.size}`,
    });
  }

  const evaluatedBucketIds = buckets.map((bucket) => bucket.id).filter(Boolean);
  const evaluatedBucketIdsJoined = evaluatedBucketIds.join('|');
  const expectedBucketOrderJoined = expectedBucketOrder.join('|');
  if (evaluatedBucketIdsJoined !== expectedBucketOrderJoined) {
    failures.push({
      id: 'runtime_closure_bucket_order_noncanonical',
      detail: `${evaluatedBucketIdsJoined || 'missing'};expected=${expectedBucketOrderJoined}`,
    });
  }

  for (const required of requiredBucketIds) {
    if (!byId.has(required)) {
      failures.push({
        id: 'runtime_closure_required_bucket_missing',
        detail: required,
      });
    }
  }

  const evaluatedBuckets = buckets.map((bucket, index) => {
    const rawBucket = bucketRowsRaw[index];
    if (!rawBucket || typeof rawBucket !== 'object' || Array.isArray(rawBucket)) {
      failures.push({
        id: 'runtime_closure_bucket_row_payload_invalid',
        detail: bucket.id || `index:${index}`,
      });
    } else {
      const rowKeys = Object.keys(rawBucket as Record<string, unknown>);
      const missing = Array.from(expectedBucketKeys).filter((key) => !rowKeys.includes(key));
      const unexpected = rowKeys.filter((key) => !expectedBucketKeys.has(key));
      if (missing.length > 0 || unexpected.length > 0) {
        failures.push({
          id: 'runtime_closure_bucket_row_keyset_drift',
          detail: `${bucket.id || `index:${index}`}:missing=${missing.join(',') || 'none'};unexpected=${unexpected.join(',') || 'none'}`,
        });
      }
      const rawId = String((rawBucket as any)?.id ?? '');
      const rawOwner = String((rawBucket as any)?.owner ?? '');
      const rawLabel = String((rawBucket as any)?.label ?? '');
      const rawStatus = String((rawBucket as any)?.status ?? '');
      if (rawId.trim() !== rawId) {
        failures.push({
          id: 'runtime_closure_bucket_id_trimmed_contract',
          detail: bucket.id || `index:${index}`,
        });
      }
      if (rawOwner.trim() !== rawOwner) {
        failures.push({
          id: 'runtime_closure_bucket_owner_trimmed_contract',
          detail: bucket.id || `index:${index}`,
        });
      }
      if (rawLabel.trim() !== rawLabel) {
        failures.push({
          id: 'runtime_closure_bucket_label_trimmed_contract',
          detail: bucket.id || `index:${index}`,
        });
      }
      if (rawStatus.trim() !== rawStatus || cleanText(rawStatus, 40) !== cleanText(rawStatus, 40).toLowerCase()) {
        failures.push({
          id: 'runtime_closure_bucket_status_lowercase_trimmed_contract',
          detail: `${bucket.id || `index:${index}`}:${cleanText(rawStatus, 40) || 'missing'}`,
        });
      }
    }
    if (!/^[a-z0-9_]+$/.test(bucket.id)) {
      failures.push({
        id: 'runtime_closure_bucket_id_noncanonical',
        detail: bucket.id || 'missing',
      });
    }
    if (!bucket.label) {
      failures.push({
        id: 'runtime_closure_bucket_label_missing',
        detail: bucket.id,
      });
    } else if (/\b(todo|tbd|placeholder|fixme)\b/i.test(bucket.label)) {
      failures.push({
        id: 'runtime_closure_bucket_label_placeholder_contract',
        detail: `${bucket.id}:${bucket.label}`,
      });
    }
    const expectedLabel = expectedBucketLabels.get(bucket.id);
    if (expectedLabel && bucket.label !== expectedLabel) {
      failures.push({
        id: 'runtime_closure_bucket_label_noncanonical_expected',
        detail: `${bucket.id}:${bucket.label || 'missing'};expected=${expectedLabel}`,
      });
    }
    if (!bucket.owner) {
      failures.push({
        id: 'runtime_closure_bucket_owner_missing',
        detail: bucket.id,
      });
    } else if (!/^[a-z0-9-]+$/.test(bucket.owner)) {
      failures.push({
        id: 'runtime_closure_bucket_owner_noncanonical',
        detail: `${bucket.id}:${bucket.owner}`,
      });
    }
    const expectedOwner = expectedBucketOwners.get(bucket.id);
    if (expectedOwner && bucket.owner !== expectedOwner) {
      failures.push({
        id: 'runtime_closure_bucket_owner_noncanonical_expected',
        detail: `${bucket.id}:${bucket.owner || 'missing'};expected=${expectedOwner}`,
      });
    }
    if (!allowedStatuses.has(bucket.status)) {
      failures.push({
        id: 'runtime_closure_bucket_status_invalid',
        detail: `${bucket.id}:${bucket.status || 'missing'}`,
      });
    }
    const expectedStatus = expectedBucketStatuses.get(bucket.id);
    if (expectedStatus && bucket.status !== expectedStatus) {
      failures.push({
        id: 'runtime_closure_bucket_status_noncanonical_expected',
        detail: `${bucket.id}:${bucket.status || 'missing'};expected=${expectedStatus}`,
      });
    }
    if (bucket.validation_gates.length === 0) {
      failures.push({
        id: 'runtime_closure_bucket_validation_gates_missing',
        detail: bucket.id,
      });
    } else {
      const duplicateValidationGates = bucket.validation_gates.filter(
        (gateId, index, arr) => arr.indexOf(gateId) !== index,
      );
      if (duplicateValidationGates.length > 0) {
        failures.push({
          id: 'runtime_closure_bucket_validation_gates_duplicate',
          detail: `${bucket.id}:${Array.from(new Set(duplicateValidationGates)).join(',')}`,
        });
      }
      const invalidValidationGateTokens = bucket.validation_gates.filter(
        (gateId) => !/^ops:[a-z0-9:-]+$/.test(gateId),
      );
      if (invalidValidationGateTokens.length > 0) {
        failures.push({
          id: 'runtime_closure_bucket_validation_gates_noncanonical',
          detail: `${bucket.id}:${invalidValidationGateTokens.join(',')}`,
        });
      }
      const sortedValidationGates = [...bucket.validation_gates].sort();
      if (sortedValidationGates.join('|') !== bucket.validation_gates.join('|')) {
        failures.push({
          id: 'runtime_closure_bucket_validation_gates_unsorted',
          detail: bucket.id,
        });
      }
    }
    const expectedValidationGates = expectedBucketValidationGates.get(bucket.id) || [];
    if (bucket.validation_gates.length !== expectedValidationGates.length) {
      failures.push({
        id: 'runtime_closure_bucket_validation_gates_count_noncanonical',
        detail: `${bucket.id}:${bucket.validation_gates.length};expected=${expectedValidationGates.length}`,
      });
    }
    if (bucket.validation_gates.join('|') !== expectedValidationGates.join('|')) {
      failures.push({
        id: 'runtime_closure_bucket_validation_gates_expected_set_mismatch',
        detail: `${bucket.id}:${bucket.validation_gates.join(',') || 'missing'};expected=${expectedValidationGates.join(',')}`,
      });
    }
    const validationGateWhitespace = bucket.validation_gates.filter((gateId) => /\s/.test(gateId));
    if (validationGateWhitespace.length > 0) {
      failures.push({
        id: 'runtime_closure_bucket_validation_gates_whitespace_token',
        detail: `${bucket.id}:${validationGateWhitespace.join(',')}`,
      });
    }
    if (bucket.evidence_artifacts.length === 0) {
      failures.push({
        id: 'runtime_closure_bucket_evidence_artifacts_missing',
        detail: bucket.id,
      });
    } else {
      const duplicateEvidenceArtifacts = bucket.evidence_artifacts.filter(
        (artifactPath, index, arr) => arr.indexOf(artifactPath) !== index,
      );
      if (duplicateEvidenceArtifacts.length > 0) {
        failures.push({
          id: 'runtime_closure_bucket_evidence_artifacts_duplicate',
          detail: `${bucket.id}:${Array.from(new Set(duplicateEvidenceArtifacts)).join(',')}`,
        });
      }
      const invalidEvidenceArtifactPaths = bucket.evidence_artifacts.filter(
        (artifactPath) =>
          !artifactPath.startsWith('core/local/artifacts/') || !artifactPath.endsWith('_current.json'),
      );
      if (invalidEvidenceArtifactPaths.length > 0) {
        failures.push({
          id: 'runtime_closure_bucket_evidence_artifacts_noncanonical',
          detail: `${bucket.id}:${invalidEvidenceArtifactPaths.join(',')}`,
        });
      }
      const sortedEvidenceArtifacts = [...bucket.evidence_artifacts].sort();
      if (sortedEvidenceArtifacts.join('|') !== bucket.evidence_artifacts.join('|')) {
        failures.push({
          id: 'runtime_closure_bucket_evidence_artifacts_unsorted',
          detail: bucket.id,
        });
      }
    }
    const expectedEvidenceArtifacts = expectedBucketEvidenceArtifacts.get(bucket.id) || [];
    if (bucket.evidence_artifacts.length !== expectedEvidenceArtifacts.length) {
      failures.push({
        id: 'runtime_closure_bucket_evidence_artifacts_count_noncanonical',
        detail: `${bucket.id}:${bucket.evidence_artifacts.length};expected=${expectedEvidenceArtifacts.length}`,
      });
    }
    if (bucket.evidence_artifacts.join('|') !== expectedEvidenceArtifacts.join('|')) {
      failures.push({
        id: 'runtime_closure_bucket_evidence_artifacts_expected_set_mismatch',
        detail: `${bucket.id}:${bucket.evidence_artifacts.join(',') || 'missing'};expected=${expectedEvidenceArtifacts.join(',')}`,
      });
    }
    const evidenceArtifactWhitespace = bucket.evidence_artifacts.filter((artifactPath) =>
      /\s/.test(artifactPath),
    );
    if (evidenceArtifactWhitespace.length > 0) {
      failures.push({
        id: 'runtime_closure_bucket_evidence_artifacts_whitespace_token',
        detail: `${bucket.id}:${evidenceArtifactWhitespace.join(',')}`,
      });
    }
    const evidenceArtifactPathTraversal = bucket.evidence_artifacts.filter(
      (artifactPath) =>
        artifactPath.includes('../') ||
        artifactPath.includes('..\\') ||
        artifactPath.startsWith('/') ||
        artifactPath.startsWith('\\'),
    );
    if (evidenceArtifactPathTraversal.length > 0) {
      failures.push({
        id: 'runtime_closure_bucket_evidence_artifacts_path_traversal',
        detail: `${bucket.id}:${evidenceArtifactPathTraversal.join(',')}`,
      });
    }
    const evidenceArtifactBasenameNoncanonical = bucket.evidence_artifacts.filter((artifactPath) => {
      const base = path.basename(artifactPath);
      return !/^[a-z0-9_]+_current\.json$/.test(base);
    });
    if (evidenceArtifactBasenameNoncanonical.length > 0) {
      failures.push({
        id: 'runtime_closure_bucket_evidence_artifact_basename_noncanonical',
        detail: `${bucket.id}:${evidenceArtifactBasenameNoncanonical.join(',')}`,
      });
    }
    const missingGateRefs = bucket.validation_gates.filter((gateId) => !knownGateIds.has(gateId));
    if (missingGateRefs.length > 0) {
      failures.push({
        id: 'runtime_closure_bucket_validation_gate_ref_unknown',
        detail: `${bucket.id}:${missingGateRefs.join(',')}`,
      });
    }
    const notInReleaseProfile = bucket.validation_gates.filter(
      (gateId) => !releaseProfileGateIds.has(gateId),
    );
    if (notInReleaseProfile.length > 0) {
      failures.push({
        id: 'runtime_closure_bucket_validation_gate_not_in_release_profile',
        detail: `${bucket.id}:${notInReleaseProfile.join(',')}`,
      });
    }
    if (runtimeCriticalBucketIds.has(bucket.id)) {
      const notInRuntimeProofProfile = bucket.validation_gates.filter(
        (gateId) => !runtimeProofProfileGateIds.has(gateId),
      );
      if (notInRuntimeProofProfile.length > 0) {
        failures.push({
          id: 'runtime_closure_runtime_critical_gate_not_in_runtime_proof_profile',
          detail: `${bucket.id}:${notInRuntimeProofProfile.join(',')}`,
        });
      }
    }
    if (bucket.evidence_artifacts.length < bucket.validation_gates.length) {
      failures.push({
        id: 'runtime_closure_bucket_evidence_artifacts_underflow',
        detail: `${bucket.id}:artifacts=${bucket.evidence_artifacts.length};gates=${bucket.validation_gates.length}`,
      });
    }
    return {
      ...bucket,
      validation_gate_count: bucket.validation_gates.length,
      evidence_artifact_count: bucket.evidence_artifacts.length,
      missing_gate_refs: missingGateRefs,
      missing_gate_refs_count: missingGateRefs.length,
    };
  });

  const duplicateLabels = duplicateTokens(
    evaluatedBuckets.map((bucket) => bucket.label).filter((label) => Boolean(label)),
  );
  if (duplicateLabels.length > 0) {
    failures.push({
      id: 'runtime_closure_bucket_labels_duplicate',
      detail: duplicateLabels.join(','),
    });
  }

  const duplicateOwners = duplicateTokens(
    evaluatedBuckets.map((bucket) => bucket.owner).filter((owner) => Boolean(owner)),
  );
  if (duplicateOwners.length > 0) {
    failures.push({
      id: 'runtime_closure_bucket_owners_duplicate',
      detail: duplicateOwners.join(','),
    });
  }

  const duplicateValidationGatesGlobal = duplicateTokens(
    evaluatedBuckets.flatMap((bucket) => bucket.validation_gates),
  ).filter((gateId) => !globallyShareableValidationGates.has(gateId));
  if (duplicateValidationGatesGlobal.length > 0) {
    failures.push({
      id: 'runtime_closure_validation_gate_global_duplicate_unapproved',
      detail: duplicateValidationGatesGlobal.join(','),
    });
  }

  const duplicateEvidenceArtifactsGlobal = duplicateTokens(
    evaluatedBuckets.flatMap((bucket) => bucket.evidence_artifacts),
  );
  if (duplicateEvidenceArtifactsGlobal.length > 0) {
    failures.push({
      id: 'runtime_closure_evidence_artifact_global_duplicate',
      detail: duplicateEvidenceArtifactsGlobal.join(','),
    });
  }

  const payload = {
    ok: failures.length === 0,
    type: 'runtime_closure_board_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    board_path: args.boardPath,
    gate_registry_path: args.gateRegistryPath,
    verify_profiles_path: args.verifyProfilesPath,
    markdown_path: args.markdownPath,
    summary: {
      pass: failures.length === 0,
      bucket_count: evaluatedBuckets.length,
      required_bucket_count: requiredBucketIds.size,
      missing_required_bucket_count: failures.filter(
        (row) => row.id === 'runtime_closure_required_bucket_missing',
      ).length,
      invalid_gate_ref_count: failures.filter(
        (row) => row.id === 'runtime_closure_bucket_validation_gate_ref_unknown',
      ).length,
      policy_failure_count: failures.filter((row) => row.id.startsWith('runtime_closure_guard_')).length,
      contract_failure_count: failures.filter((row) => row.id.startsWith('runtime_closure_bucket_')).length,
      failure_count: failures.length,
    },
    buckets: evaluatedBuckets,
    failures,
  };

  writeMarkdown(path.resolve(root, args.markdownPath), renderMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outPath,
    strict: args.strict,
    ok: payload.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
