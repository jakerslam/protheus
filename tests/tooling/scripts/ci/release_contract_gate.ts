#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { executeGate } from '../../lib/runner.ts';

type Check = {
  id: string;
  ok: boolean;
  detail: string;
};

const ROOT = process.cwd();
const DEFAULT_OUT = path.join(ROOT, 'core/local/artifacts/release_contract_gate_current.json');
const GATE_REGISTRY_PATH = 'tests/tooling/config/tooling_gate_registry.json';
const TS_ENTRYPOINT = path.join(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const TOPOLOGY_STATUS_SCRIPT = path.join(ROOT, 'client/runtime/systems/ops/transport_topology_status.ts');
const RELEASE_CHANNEL_POLICY_PATH = 'client/runtime/config/release_channel_policy.json';
const RELEASE_COMPATIBILITY_POLICY_PATH = 'client/runtime/config/release_compatibility_policy.json';
const SCHEMA_VERSIONING_POLICY_PATH = 'client/runtime/config/schema_versioning_gate_policy.json';
const DEPENDENCY_UPDATE_POLICY_PATH = 'client/runtime/config/dependency_update_policy.json';
const API_CLI_REGISTRY_PATH = 'client/runtime/config/api_cli_contract_registry.json';
const TOOLING_GATE_REGISTRY_PATH = 'tests/tooling/config/tooling_gate_registry.json';
const VERIFY_PROFILES_PATH = 'tests/tooling/config/verify_profiles.json';
const RELEASE_WORKFLOW_PATH = '.github/workflows/release.yml';
const RELEASE_PROOF_PACK_MANIFEST_PATH = 'tests/tooling/config/release_proof_pack_manifest.json';
const WRAPPER_FILES = [
  'client/runtime/systems/autonomy/self_improvement_cadence_orchestrator.ts',
  'client/runtime/systems/memory/causal_temporal_graph.ts',
  'client/runtime/systems/execution/task_decomposition_primitive.ts',
  'client/runtime/systems/workflow/universal_outreach_primitive.ts',
];

function parseArgs(argv: string[]) {
  return {
    strict: argv.includes('--strict=1') || argv.includes('--strict'),
    out: argv.find((token) => token.startsWith('--out='))?.slice('--out='.length) || DEFAULT_OUT,
  };
}

function read(relPath: string): string {
  return fs.readFileSync(path.join(ROOT, relPath), 'utf8');
}

function parseJson(relPath: string): { ok: true; value: any } | { ok: false; detail: string } {
  try {
    return { ok: true, value: JSON.parse(read(relPath)) };
  } catch (error: any) {
    return {
      ok: false,
      detail: `${relPath}:parse_error:${String(error?.message || error || 'unknown')}`,
    };
  }
}

function asTrimmedString(value: unknown): string {
  return typeof value === 'string' ? value.trim() : '';
}

function isNonEmptyString(value: unknown): boolean {
  return asTrimmedString(value).length > 0;
}

function isSemverToken(value: unknown): boolean {
  return /^\d+\.\d+\.\d+$/.test(asTrimmedString(value));
}

function isCanonicalRelativePathToken(
  value: string,
  requiredPrefix = '',
  requiredSuffix = '',
): boolean {
  const normalized = asTrimmedString(value);
  if (!normalized) return false;
  if (path.isAbsolute(normalized)) return false;
  if (normalized.includes('\\')) return false;
  if (normalized.includes('..')) return false;
  if (normalized.includes('//')) return false;
  if (/\s/.test(normalized)) return false;
  if (requiredPrefix && !normalized.startsWith(requiredPrefix)) return false;
  if (requiredSuffix && !normalized.endsWith(requiredSuffix)) return false;
  return true;
}

function releaseContractPathAndConstantChecks(args: ReturnType<typeof parseArgs>): Check[] {
  const outRaw = asTrimmedString(args.out) || DEFAULT_OUT;
  const outAbs = path.resolve(ROOT, outRaw);
  const artifactsRoot = path.resolve(ROOT, 'core/local/artifacts');
  const outRelFromArtifacts = path.relative(artifactsRoot, outAbs).replace(/\\/g, '/');
  const outInsideArtifacts =
    outRelFromArtifacts.length > 0
    && outRelFromArtifacts !== '..'
    && !outRelFromArtifacts.startsWith('../')
    && !path.isAbsolute(outRelFromArtifacts);
  const policyPaths = [
    RELEASE_CHANNEL_POLICY_PATH,
    RELEASE_COMPATIBILITY_POLICY_PATH,
    SCHEMA_VERSIONING_POLICY_PATH,
    DEPENDENCY_UPDATE_POLICY_PATH,
    API_CLI_REGISTRY_PATH,
  ];
  const policyPathDuplicates = policyPaths.filter((row, idx, arr) => arr.indexOf(row) !== idx);
  const policyPathsCanonical = policyPaths.every((row) =>
    isCanonicalRelativePathToken(row, 'client/runtime/config/', '.json')
  );
  return [
    {
      id: 'release_contract_gate_out_path_canonical_contract',
      ok: isNonEmptyString(outRaw) && !outRaw.includes('\0'),
      detail: outRaw || 'missing',
    },
    {
      id: 'release_contract_gate_out_path_json_suffix_contract',
      ok: outAbs.endsWith('.json'),
      detail: outRaw || 'missing',
    },
    {
      id: 'release_contract_gate_out_path_current_suffix_contract',
      ok: outAbs.endsWith('_current.json'),
      detail: outRaw || 'missing',
    },
    {
      id: 'release_contract_gate_out_path_artifacts_scope_contract',
      ok: outInsideArtifacts,
      detail: outRaw || 'missing',
    },
    {
      id: 'release_contract_gate_registry_path_constant_alignment_contract',
      ok: GATE_REGISTRY_PATH === TOOLING_GATE_REGISTRY_PATH,
      detail: `gate=${GATE_REGISTRY_PATH};tooling=${TOOLING_GATE_REGISTRY_PATH}`,
    },
    {
      id: 'release_contract_policy_paths_unique_contract',
      ok: policyPathDuplicates.length === 0,
      detail:
        policyPathDuplicates.length === 0
          ? 'ok'
          : Array.from(new Set(policyPathDuplicates)).join(','),
    },
    {
      id: 'release_contract_policy_paths_canonical_contract',
      ok: policyPathsCanonical,
      detail: policyPaths.join(','),
    },
    {
      id: 'release_contract_release_workflow_path_canonical_contract',
      ok:
        RELEASE_WORKFLOW_PATH === '.github/workflows/release.yml'
        && isCanonicalRelativePathToken(RELEASE_WORKFLOW_PATH, '.github/workflows/', '.yml'),
      detail: RELEASE_WORKFLOW_PATH,
    },
    {
      id: 'release_contract_proof_pack_manifest_path_canonical_contract',
      ok:
        RELEASE_PROOF_PACK_MANIFEST_PATH === 'tests/tooling/config/release_proof_pack_manifest.json'
        && isCanonicalRelativePathToken(
          RELEASE_PROOF_PACK_MANIFEST_PATH,
          'tests/tooling/config/',
          '.json',
        ),
      detail: RELEASE_PROOF_PACK_MANIFEST_PATH,
    },
    {
      id: 'release_contract_verify_profiles_path_canonical_contract',
      ok:
        VERIFY_PROFILES_PATH === 'tests/tooling/config/verify_profiles.json'
        && isCanonicalRelativePathToken(VERIFY_PROFILES_PATH, 'tests/tooling/config/', '.json'),
      detail: VERIFY_PROFILES_PATH,
    },
  ];
}

function releaseContractFilePresenceChecks(): Check[] {
  const policyPaths = [
    RELEASE_CHANNEL_POLICY_PATH,
    RELEASE_COMPATIBILITY_POLICY_PATH,
    SCHEMA_VERSIONING_POLICY_PATH,
    DEPENDENCY_UPDATE_POLICY_PATH,
    API_CLI_REGISTRY_PATH,
  ];
  const missingPolicyFiles = policyPaths.filter((row) => !fs.existsSync(path.join(ROOT, row)));
  return [
    {
      id: 'release_contract_policy_files_exist_contract',
      ok: missingPolicyFiles.length === 0,
      detail: missingPolicyFiles.length === 0 ? 'ok' : missingPolicyFiles.join(','),
    },
    {
      id: 'release_contract_release_workflow_exists_contract',
      ok: fs.existsSync(path.join(ROOT, RELEASE_WORKFLOW_PATH)),
      detail: RELEASE_WORKFLOW_PATH,
    },
    {
      id: 'release_contract_proof_pack_manifest_exists_contract',
      ok: fs.existsSync(path.join(ROOT, RELEASE_PROOF_PACK_MANIFEST_PATH)),
      detail: RELEASE_PROOF_PACK_MANIFEST_PATH,
    },
    {
      id: 'release_contract_verify_profiles_exists_contract',
      ok: fs.existsSync(path.join(ROOT, VERIFY_PROFILES_PATH)),
      detail: VERIFY_PROFILES_PATH,
    },
    {
      id: 'release_contract_gate_registry_exists_contract',
      ok: fs.existsSync(path.join(ROOT, TOOLING_GATE_REGISTRY_PATH)),
      detail: TOOLING_GATE_REGISTRY_PATH,
    },
  ];
}

function releaseContractWrapperListChecks(): Check[] {
  const duplicates = WRAPPER_FILES.filter((row, idx, arr) => arr.indexOf(row) !== idx);
  const nonCanonical = WRAPPER_FILES.filter((row) =>
    !isCanonicalRelativePathToken(row, 'client/runtime/systems/', '.ts')
  );
  const missing = WRAPPER_FILES.filter((row) => !fs.existsSync(path.join(ROOT, row)));
  return [
    {
      id: 'release_contract_wrapper_files_nonempty_contract',
      ok: WRAPPER_FILES.length > 0,
      detail: String(WRAPPER_FILES.length),
    },
    {
      id: 'release_contract_wrapper_files_unique_contract',
      ok: duplicates.length === 0,
      detail: duplicates.length === 0 ? 'ok' : Array.from(new Set(duplicates)).join(','),
    },
    {
      id: 'release_contract_wrapper_files_canonical_contract',
      ok: nonCanonical.length === 0,
      detail: nonCanonical.length === 0 ? 'ok' : nonCanonical.join(','),
    },
    {
      id: 'release_contract_wrapper_files_exist_contract',
      ok: missing.length === 0,
      detail: missing.length === 0 ? 'ok' : missing.join(','),
    },
  ];
}

function objectKeysetViolations(
  value: unknown,
  requiredKeys: string[],
): { missing: string[]; unexpected: string[] } {
  const keys =
    value && typeof value === 'object' && !Array.isArray(value)
      ? Object.keys(value as Record<string, unknown>)
      : [];
  const required = new Set(requiredKeys);
  const missing = requiredKeys.filter((key) => !keys.includes(key));
  const unexpected = keys.filter((key) => !required.has(key));
  return { missing, unexpected };
}

function runGateCheck(id: string): Check {
  const out = executeGate(id, {
    registryPath: GATE_REGISTRY_PATH,
    strict: true,
  });
  return {
    id,
    ok: out.ok,
    detail: out.ok
      ? 'ok'
      : String(out.failures[0]?.detail || `status=${out.summary.exit_code}`).slice(0, 500),
  };
}

function releaseChannelPolicySchemaContractCheck(): Check {
  const parsed = parseJson(RELEASE_CHANNEL_POLICY_PATH);
  if (!parsed.ok) return { id: 'release_channel_policy_schema_contract', ok: false, detail: parsed.detail };
  const policy = parsed.value || {};
  const channels = Array.isArray(policy.channels) ? policy.channels : [];
  const canonicalChannels = ['alpha', 'beta', 'stable'];
  const violations: string[] = [];
  if (policy.schema_id !== 'release_channel_policy') violations.push('schema_id');
  if (policy.schema_version !== '1.0') violations.push('schema_version');
  if (policy.default_channel !== 'alpha') violations.push('default_channel');
  if (channels.length !== canonicalChannels.length) violations.push('channels:length');
  if (new Set(channels).size !== channels.length) violations.push('channels:duplicate');
  if (!channels.every((row: any, idx: number) => row === canonicalChannels[idx])) violations.push('channels:order_or_token');
  return {
    id: 'release_channel_policy_schema_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function releaseChannelPromotionContractCheck(): Check {
  const parsed = parseJson(RELEASE_CHANNEL_POLICY_PATH);
  if (!parsed.ok) return { id: 'release_channel_promotion_contract', ok: false, detail: parsed.detail };
  const policy = parsed.value || {};
  const channels = Array.isArray(policy.channels) ? policy.channels : [];
  const promotions = Array.isArray(policy.promotion_rules) ? policy.promotion_rules : [];
  const canonicalRules = ['alpha->beta', 'beta->stable', 'alpha->stable'];
  const encodedRules = promotions.map((row: any) => `${String(row?.from || '')}->${String(row?.to || '')}`);
  const violations: string[] = [];
  if (encodedRules.length !== canonicalRules.length) violations.push('promotion_rules:length');
  if (new Set(encodedRules).size !== encodedRules.length) violations.push('promotion_rules:duplicate');
  if (!encodedRules.every((row: string, idx: number) => row === canonicalRules[idx])) violations.push('promotion_rules:order_or_token');
  if (!promotions.every((row: any) => channels.includes(row?.from) && channels.includes(row?.to))) {
    violations.push('promotion_rules:unknown_channel');
  }
  return {
    id: 'release_channel_promotion_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function releaseCompatibilityPolicyContractCheck(): Check {
  const parsed = parseJson(RELEASE_COMPATIBILITY_POLICY_PATH);
  if (!parsed.ok) return { id: 'release_compatibility_policy_contract', ok: false, detail: parsed.detail };
  const policy = parsed.value || {};
  const violations: string[] = [];
  if (policy.schema_id !== 'release_compatibility_policy') violations.push('schema_id');
  if (policy.schema_version !== '1.0') violations.push('schema_version');
  if (!(Number.isInteger(policy.required_deprecation_days) && policy.required_deprecation_days >= 90)) {
    violations.push('required_deprecation_days');
  }
  if (policy.require_migration_guide !== true) violations.push('require_migration_guide');
  if (policy.require_deprecation_notice !== true) violations.push('require_deprecation_notice');
  if (policy.registry_path !== API_CLI_REGISTRY_PATH) violations.push('registry_path');
  return {
    id: 'release_compatibility_policy_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function apiCliContractRegistrySchemaContractCheck(): Check {
  const parsed = parseJson(API_CLI_REGISTRY_PATH);
  if (!parsed.ok) return { id: 'api_cli_registry_schema_contract', ok: false, detail: parsed.detail };
  const registry = parsed.value || {};
  const violations: string[] = [];
  if (registry.schema_id !== 'api_cli_contract_registry') violations.push('schema_id');
  if (registry.schema_version !== '1.0') violations.push('schema_version');
  const apiContracts = Array.isArray(registry.api_contracts) ? registry.api_contracts : [];
  const cliContracts = Array.isArray(registry.cli_contracts) ? registry.cli_contracts : [];
  if (apiContracts.length === 0) violations.push('api_contracts:empty');
  if (cliContracts.length === 0) violations.push('cli_contracts:empty');
  const allNames = [...apiContracts, ...cliContracts].map((row: any) => asTrimmedString(row?.name));
  if (allNames.some((row: string) => row.length === 0)) violations.push('contracts:name_empty');
  if (new Set(allNames).size !== allNames.length) violations.push('contracts:name_duplicate');
  const hasInvalidStatus = [...apiContracts, ...cliContracts].some((row: any) => {
    const status = asTrimmedString(row?.status);
    return status !== 'active' && status !== 'deprecated';
  });
  if (hasInvalidStatus) violations.push('contracts:status');
  return {
    id: 'api_cli_registry_schema_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function apiCliContractRegistryDeprecationContractCheck(): Check {
  const compatibilityParsed = parseJson(RELEASE_COMPATIBILITY_POLICY_PATH);
  if (!compatibilityParsed.ok) return { id: 'api_cli_registry_deprecation_contract', ok: false, detail: compatibilityParsed.detail };
  const registryParsed = parseJson(API_CLI_REGISTRY_PATH);
  if (!registryParsed.ok) return { id: 'api_cli_registry_deprecation_contract', ok: false, detail: registryParsed.detail };
  const compatibility = compatibilityParsed.value || {};
  const registry = registryParsed.value || {};
  const minimumWindow = Number(compatibility.required_deprecation_days || 0);
  const rows = [...(Array.isArray(registry.api_contracts) ? registry.api_contracts : []), ...(Array.isArray(registry.cli_contracts) ? registry.cli_contracts : [])];
  const deprecatedRows = rows.filter((row: any) => asTrimmedString(row?.status) === 'deprecated');
  const violations: string[] = [];
  if (deprecatedRows.length === 0) violations.push('deprecated_contracts:missing');
  for (const row of deprecatedRows) {
    const name = asTrimmedString(row?.name) || '<unknown>';
    if (compatibility.require_deprecation_notice === true && !isNonEmptyString(row?.deprecation_notice)) {
      violations.push(`${name}:deprecation_notice`);
    }
    if (compatibility.require_migration_guide === true && !isNonEmptyString(row?.migration_guide)) {
      violations.push(`${name}:migration_guide`);
    }
    if (!(Number.isInteger(row?.deprecation_window_days) && row.deprecation_window_days >= minimumWindow)) {
      violations.push(`${name}:deprecation_window_days`);
    }
  }
  return {
    id: 'api_cli_registry_deprecation_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function schemaVersioningGatePolicyContractCheck(): Check {
  const parsed = parseJson(SCHEMA_VERSIONING_POLICY_PATH);
  if (!parsed.ok) return { id: 'schema_versioning_gate_policy_contract', ok: false, detail: parsed.detail };
  const policy = parsed.value || {};
  const targets = Array.isArray(policy.targets) ? policy.targets : [];
  const targetIds = targets.map((row: any) => asTrimmedString(row?.id));
  const violations: string[] = [];
  if (policy.version !== '1.0') violations.push('version');
  if (policy.enabled !== true) violations.push('enabled');
  if (targets.length === 0) violations.push('targets:empty');
  if (targetIds.some((row: string) => row.length === 0)) violations.push('targets:id_empty');
  if (new Set(targetIds).size !== targetIds.length) violations.push('targets:id_duplicate');
  const badTargetShape = targets.some((row: any) => {
    return (
      !isNonEmptyString(row?.path) ||
      !isNonEmptyString(row?.required_schema_id) ||
      !isNonEmptyString(row?.min_schema_version) ||
      row?.kind !== 'json'
    );
  });
  if (badTargetShape) violations.push('targets:shape');
  const migrations = policy.migrations || {};
  if (migrations.target_default_version !== '1.0') violations.push('migrations:target_default_version');
  if (migrations.allow_add_missing_fields_only !== true) violations.push('migrations:allow_add_missing_fields_only');
  return {
    id: 'schema_versioning_gate_policy_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function schemaVersioningGateOutputsContractCheck(): Check {
  const parsed = parseJson(SCHEMA_VERSIONING_POLICY_PATH);
  if (!parsed.ok) return { id: 'schema_versioning_gate_outputs_contract', ok: false, detail: parsed.detail };
  const outputs = parsed.value?.outputs || {};
  const latest = asTrimmedString(outputs.latest_path);
  const history = asTrimmedString(outputs.history_path);
  const violations: string[] = [];
  if (!latest.startsWith('local/state/contracts/schema_versioning_gate/')) violations.push('outputs:latest_path_prefix');
  if (!latest.endsWith('/latest.json') && !latest.endsWith('latest.json')) violations.push('outputs:latest_path_suffix');
  if (!history.startsWith('local/state/contracts/schema_versioning_gate/')) violations.push('outputs:history_path_prefix');
  if (!history.endsWith('/history.jsonl') && !history.endsWith('history.jsonl')) violations.push('outputs:history_path_suffix');
  if (latest.length === 0 || history.length === 0) violations.push('outputs:missing');
  return {
    id: 'schema_versioning_gate_outputs_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function dependencyUpdatePolicyContractCheck(): Check {
  const parsed = parseJson(DEPENDENCY_UPDATE_POLICY_PATH);
  if (!parsed.ok) return { id: 'dependency_update_policy_contract', ok: false, detail: parsed.detail };
  const policy = parsed.value || {};
  const ecosystems = Array.isArray(policy.dependabot_required_ecosystems) ? policy.dependabot_required_ecosystems : [];
  const canonical = ['npm', 'cargo', 'github-actions'];
  const violations: string[] = [];
  if (policy.schema_id !== 'dependency_update_policy') violations.push('schema_id');
  if (policy.schema_version !== '1.0') violations.push('schema_version');
  if (!(Number.isInteger(policy.security_patch_sla_days) && policy.security_patch_sla_days > 0 && policy.security_patch_sla_days <= 14)) {
    violations.push('security_patch_sla_days');
  }
  if (policy.max_critical_vulnerabilities !== 0) violations.push('max_critical_vulnerabilities');
  if (policy.max_high_vulnerabilities !== 0) violations.push('max_high_vulnerabilities');
  if (ecosystems.length !== canonical.length) violations.push('dependabot_required_ecosystems:length');
  if (new Set(ecosystems).size !== ecosystems.length) violations.push('dependabot_required_ecosystems:duplicate');
  if (!canonical.every((row) => ecosystems.includes(row))) violations.push('dependabot_required_ecosystems:missing');
  return {
    id: 'dependency_update_policy_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function dependencyUpdateBlocklistContractCheck(): Check {
  const parsed = parseJson(DEPENDENCY_UPDATE_POLICY_PATH);
  if (!parsed.ok) return { id: 'dependency_update_blocklist_contract', ok: false, detail: parsed.detail };
  const policy = parsed.value || {};
  const blocked = Array.isArray(policy.blocked_packages) ? policy.blocked_packages : [];
  const allowedEcosystems = new Set(
    Array.isArray(policy.dependabot_required_ecosystems) ? policy.dependabot_required_ecosystems : [],
  );
  const signatures = blocked.map((row: any) => `${asTrimmedString(row?.ecosystem)}:${asTrimmedString(row?.name)}`);
  const violations: string[] = [];
  if (blocked.length === 0) violations.push('blocked_packages:empty');
  if (new Set(signatures).size !== signatures.length) violations.push('blocked_packages:duplicate');
  const badRows = blocked.some((row: any) => {
    const ecosystem = asTrimmedString(row?.ecosystem);
    const name = asTrimmedString(row?.name);
    const reason = asTrimmedString(row?.reason);
    return !allowedEcosystems.has(ecosystem) || name.length === 0 || reason.length === 0;
  });
  if (badRows) violations.push('blocked_packages:shape_or_ecosystem');
  return {
    id: 'dependency_update_blocklist_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function releaseChannelPolicyKeysetContractCheck(): Check {
  const parsed = parseJson(RELEASE_CHANNEL_POLICY_PATH);
  if (!parsed.ok) return { id: 'release_channel_policy_keyset_contract', ok: false, detail: parsed.detail };
  const policy = parsed.value || {};
  const { missing, unexpected } = objectKeysetViolations(policy, [
    'schema_id',
    'schema_version',
    'default_channel',
    'channels',
    'promotion_rules',
  ]);
  const violations: string[] = [];
  if (missing.length > 0) violations.push(`missing=${missing.join(',')}`);
  if (unexpected.length > 0) violations.push(`unexpected=${unexpected.join(',')}`);
  return {
    id: 'release_channel_policy_keyset_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function releaseChannelPolicyChannelTokenFormatContractCheck(): Check {
  const parsed = parseJson(RELEASE_CHANNEL_POLICY_PATH);
  if (!parsed.ok) return { id: 'release_channel_policy_channel_token_format_contract', ok: false, detail: parsed.detail };
  const policy = parsed.value || {};
  const channels = Array.isArray(policy.channels) ? policy.channels : [];
  const violations: string[] = [];
  const invalidChannels = channels
    .map((row: unknown) => asTrimmedString(row))
    .filter((row: string) => !/^[a-z][a-z0-9-]*$/.test(row));
  if (invalidChannels.length > 0) violations.push(`channels=${invalidChannels.join(',')}`);
  if (!channels.includes(policy.default_channel)) violations.push('default_channel_not_in_channels');
  return {
    id: 'release_channel_policy_channel_token_format_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function releaseChannelPromotionRuleShapeContractCheck(): Check {
  const parsed = parseJson(RELEASE_CHANNEL_POLICY_PATH);
  if (!parsed.ok) return { id: 'release_channel_promotion_rule_shape_contract', ok: false, detail: parsed.detail };
  const policy = parsed.value || {};
  const channels = new Set(Array.isArray(policy.channels) ? policy.channels.map((row: unknown) => asTrimmedString(row)) : []);
  const rows = Array.isArray(policy.promotion_rules) ? policy.promotion_rules : [];
  const signatures = rows.map((row: any) => `${asTrimmedString(row?.from)}->${asTrimmedString(row?.to)}`);
  const duplicates = signatures.filter((row, idx, arr) => arr.indexOf(row) !== idx);
  const violations: string[] = [];
  for (const row of rows) {
    const from = asTrimmedString(row?.from);
    const to = asTrimmedString(row?.to);
    if (!(from && to)) violations.push('empty_from_or_to');
    if (from === to) violations.push(`self_edge=${from}`);
    if (!channels.has(from) || !channels.has(to)) violations.push(`unknown_channel=${from}->${to}`);
    const keys = row && typeof row === 'object' && !Array.isArray(row) ? Object.keys(row) : [];
    const missing = ['from', 'to'].filter((key) => !keys.includes(key));
    const unexpected = keys.filter((key) => key !== 'from' && key !== 'to');
    if (missing.length > 0) violations.push(`missing_keys=${missing.join(',')}`);
    if (unexpected.length > 0) violations.push(`unexpected_keys=${unexpected.join(',')}`);
  }
  if (duplicates.length > 0) violations.push(`duplicate_edges=${Array.from(new Set(duplicates)).join(',')}`);
  return {
    id: 'release_channel_promotion_rule_shape_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function releaseCompatibilityPolicyKeysetContractCheck(): Check {
  const parsed = parseJson(RELEASE_COMPATIBILITY_POLICY_PATH);
  if (!parsed.ok) return { id: 'release_compatibility_policy_keyset_contract', ok: false, detail: parsed.detail };
  const policy = parsed.value || {};
  const { missing, unexpected } = objectKeysetViolations(policy, [
    'schema_id',
    'schema_version',
    'required_deprecation_days',
    'require_migration_guide',
    'require_deprecation_notice',
    'registry_path',
  ]);
  const violations: string[] = [];
  if (missing.length > 0) violations.push(`missing=${missing.join(',')}`);
  if (unexpected.length > 0) violations.push(`unexpected=${unexpected.join(',')}`);
  return {
    id: 'release_compatibility_policy_keyset_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function releaseCompatibilityRegistryPathFormatContractCheck(): Check {
  const parsed = parseJson(RELEASE_COMPATIBILITY_POLICY_PATH);
  if (!parsed.ok) return { id: 'release_compatibility_registry_path_format_contract', ok: false, detail: parsed.detail };
  const policy = parsed.value || {};
  const registryPath = asTrimmedString(policy.registry_path);
  const violations: string[] = [];
  if (!registryPath.startsWith('client/runtime/config/')) violations.push('registry_path_prefix');
  if (!registryPath.endsWith('.json')) violations.push('registry_path_suffix');
  if (registryPath.includes('..') || /\s/.test(registryPath)) violations.push('registry_path_noncanonical');
  return {
    id: 'release_compatibility_registry_path_format_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function apiCliRegistryContractVersionSemverContractCheck(): Check {
  const parsed = parseJson(API_CLI_REGISTRY_PATH);
  if (!parsed.ok) return { id: 'api_cli_registry_contract_version_semver_contract', ok: false, detail: parsed.detail };
  const registry = parsed.value || {};
  const rows = [
    ...(Array.isArray(registry.api_contracts) ? registry.api_contracts : []),
    ...(Array.isArray(registry.cli_contracts) ? registry.cli_contracts : []),
  ];
  const violations = rows
    .filter((row: any) => !isSemverToken(row?.version))
    .map((row: any) => `${asTrimmedString(row?.name) || '<unknown>'}:${asTrimmedString(row?.version) || 'missing'}`);
  return {
    id: 'api_cli_registry_contract_version_semver_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function apiCliRegistryContractNameTokenContractCheck(): Check {
  const parsed = parseJson(API_CLI_REGISTRY_PATH);
  if (!parsed.ok) return { id: 'api_cli_registry_contract_name_token_contract', ok: false, detail: parsed.detail };
  const registry = parsed.value || {};
  const rows = [
    ...(Array.isArray(registry.api_contracts) ? registry.api_contracts : []),
    ...(Array.isArray(registry.cli_contracts) ? registry.cli_contracts : []),
  ];
  const violations = rows
    .filter((row: any) => !/^[a-z0-9][a-z0-9._-]*$/.test(asTrimmedString(row?.name)))
    .map((row: any) => asTrimmedString(row?.name) || '<missing>');
  return {
    id: 'api_cli_registry_contract_name_token_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function schemaVersioningTargetIdTokenContractCheck(): Check {
  const parsed = parseJson(SCHEMA_VERSIONING_POLICY_PATH);
  if (!parsed.ok) return { id: 'schema_versioning_target_id_token_contract', ok: false, detail: parsed.detail };
  const targets = Array.isArray(parsed.value?.targets) ? parsed.value.targets : [];
  const violations = targets
    .filter((row: any) => !/^[a-z0-9_]+$/.test(asTrimmedString(row?.id)))
    .map((row: any) => asTrimmedString(row?.id) || '<missing>');
  return {
    id: 'schema_versioning_target_id_token_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function schemaVersioningTargetPathContractCheck(): Check {
  const parsed = parseJson(SCHEMA_VERSIONING_POLICY_PATH);
  if (!parsed.ok) return { id: 'schema_versioning_target_path_contract', ok: false, detail: parsed.detail };
  const targets = Array.isArray(parsed.value?.targets) ? parsed.value.targets : [];
  const violations = targets
    .filter((row: any) => {
      const targetPath = asTrimmedString(row?.path);
      return (
        !targetPath.startsWith('client/runtime/config/contracts/')
        || !targetPath.endsWith('.schema.json')
        || targetPath.includes('..')
        || /\s/.test(targetPath)
      );
    })
    .map((row: any) => `${asTrimmedString(row?.id) || '<unknown>'}:${asTrimmedString(row?.path) || 'missing'}`);
  return {
    id: 'schema_versioning_target_path_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function schemaVersioningTargetSchemaIdAlignmentContractCheck(): Check {
  const parsed = parseJson(SCHEMA_VERSIONING_POLICY_PATH);
  if (!parsed.ok) return { id: 'schema_versioning_target_schema_id_alignment_contract', ok: false, detail: parsed.detail };
  const targets = Array.isArray(parsed.value?.targets) ? parsed.value.targets : [];
  const violations = targets
    .filter((row: any) => asTrimmedString(row?.id) !== asTrimmedString(row?.required_schema_id))
    .map((row: any) => `${asTrimmedString(row?.id) || '<missing>'}:${asTrimmedString(row?.required_schema_id) || 'missing'}`);
  return {
    id: 'schema_versioning_target_schema_id_alignment_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function schemaVersioningTargetVersionSemverContractCheck(): Check {
  const parsed = parseJson(SCHEMA_VERSIONING_POLICY_PATH);
  if (!parsed.ok) return { id: 'schema_versioning_target_version_semver_contract', ok: false, detail: parsed.detail };
  const targets = Array.isArray(parsed.value?.targets) ? parsed.value.targets : [];
  const violations = targets
    .filter((row: any) => !isSemverToken(row?.min_schema_version))
    .map((row: any) => `${asTrimmedString(row?.id) || '<missing>'}:${asTrimmedString(row?.min_schema_version) || 'missing'}`);
  return {
    id: 'schema_versioning_target_version_semver_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function schemaVersioningOutputsKeysetContractCheck(): Check {
  const parsed = parseJson(SCHEMA_VERSIONING_POLICY_PATH);
  if (!parsed.ok) return { id: 'schema_versioning_outputs_keyset_contract', ok: false, detail: parsed.detail };
  const outputs = parsed.value?.outputs || {};
  const { missing, unexpected } = objectKeysetViolations(outputs, ['latest_path', 'history_path']);
  const violations: string[] = [];
  if (missing.length > 0) violations.push(`missing=${missing.join(',')}`);
  if (unexpected.length > 0) violations.push(`unexpected=${unexpected.join(',')}`);
  return {
    id: 'schema_versioning_outputs_keyset_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function dependencyUpdatePolicyKeysetContractCheck(): Check {
  const parsed = parseJson(DEPENDENCY_UPDATE_POLICY_PATH);
  if (!parsed.ok) return { id: 'dependency_update_policy_keyset_contract', ok: false, detail: parsed.detail };
  const policy = parsed.value || {};
  const { missing, unexpected } = objectKeysetViolations(policy, [
    'schema_id',
    'schema_version',
    'security_patch_sla_days',
    'max_critical_vulnerabilities',
    'max_high_vulnerabilities',
    'dependabot_required_ecosystems',
    'blocked_packages',
  ]);
  const violations: string[] = [];
  if (missing.length > 0) violations.push(`missing=${missing.join(',')}`);
  if (unexpected.length > 0) violations.push(`unexpected=${unexpected.join(',')}`);
  return {
    id: 'dependency_update_policy_keyset_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function dependencyUpdateBlockedPackageNameTokenContractCheck(): Check {
  const parsed = parseJson(DEPENDENCY_UPDATE_POLICY_PATH);
  if (!parsed.ok) return { id: 'dependency_update_blocked_package_name_token_contract', ok: false, detail: parsed.detail };
  const policy = parsed.value || {};
  const blocked = Array.isArray(policy.blocked_packages) ? policy.blocked_packages : [];
  const violations = blocked
    .filter((row: any) => !/^[a-z0-9][a-z0-9._-]*$/.test(asTrimmedString(row?.name)))
    .map((row: any) => `${asTrimmedString(row?.ecosystem) || '<missing>'}:${asTrimmedString(row?.name) || '<missing>'}`);
  return {
    id: 'dependency_update_blocked_package_name_token_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function dependencyUpdateBlockedPackageReasonQualityContractCheck(): Check {
  const parsed = parseJson(DEPENDENCY_UPDATE_POLICY_PATH);
  if (!parsed.ok) return { id: 'dependency_update_blocked_package_reason_quality_contract', ok: false, detail: parsed.detail };
  const policy = parsed.value || {};
  const blocked = Array.isArray(policy.blocked_packages) ? policy.blocked_packages : [];
  const violations = blocked
    .filter((row: any) => asTrimmedString(row?.reason).length < 12)
    .map((row: any) => `${asTrimmedString(row?.ecosystem) || '<missing>'}:${asTrimmedString(row?.name) || '<missing>'}`);
  return {
    id: 'dependency_update_blocked_package_reason_quality_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function verifyProfilesReleaseGateUniquenessContractCheck(): Check {
  const parsed = parseJson(VERIFY_PROFILES_PATH);
  if (!parsed.ok) return { id: 'verify_profiles_release_gate_uniqueness_contract', ok: false, detail: parsed.detail };
  const gateIds = Array.isArray(parsed.value?.profiles?.release?.gate_ids) ? parsed.value.profiles.release.gate_ids : [];
  const duplicates = gateIds.filter((row: string, idx: number, arr: string[]) => arr.indexOf(row) !== idx);
  return {
    id: 'verify_profiles_release_gate_uniqueness_contract',
    ok: duplicates.length === 0,
    detail: duplicates.length === 0 ? 'ok' : `duplicates=${Array.from(new Set(duplicates)).join(',')}`,
  };
}

function verifyProfilesRuntimeProofGateUniquenessContractCheck(): Check {
  const parsed = parseJson(VERIFY_PROFILES_PATH);
  if (!parsed.ok) return { id: 'verify_profiles_runtime_proof_gate_uniqueness_contract', ok: false, detail: parsed.detail };
  const gateIds = Array.isArray(parsed.value?.profiles?.['runtime-proof']?.gate_ids)
    ? parsed.value.profiles['runtime-proof'].gate_ids
    : [];
  const duplicates = gateIds.filter((row: string, idx: number, arr: string[]) => arr.indexOf(row) !== idx);
  return {
    id: 'verify_profiles_runtime_proof_gate_uniqueness_contract',
    ok: duplicates.length === 0,
    detail: duplicates.length === 0 ? 'ok' : `duplicates=${Array.from(new Set(duplicates)).join(',')}`,
  };
}

function releaseProofPackManifestCategoryKeysetContractCheck(): Check {
  const parsed = parseJson(RELEASE_PROOF_PACK_MANIFEST_PATH);
  if (!parsed.ok) return { id: 'release_proof_pack_manifest_category_keyset_contract', ok: false, detail: parsed.detail };
  const groups = parsed.value?.artifact_groups || {};
  const { missing, unexpected } = objectKeysetViolations(groups, [
    'runtime_proof',
    'adapter_and_orchestration',
    'release_governance',
    'workload_and_quality',
  ]);
  const violations: string[] = [];
  if (missing.length > 0) violations.push(`missing=${missing.join(',')}`);
  if (unexpected.length > 0) violations.push(`unexpected=${unexpected.join(',')}`);
  return {
    id: 'release_proof_pack_manifest_category_keyset_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function releaseProofPackManifestRequiredArtifactUniquenessContractCheck(): Check {
  const parsed = parseJson(RELEASE_PROOF_PACK_MANIFEST_PATH);
  if (!parsed.ok) return { id: 'release_proof_pack_manifest_required_artifact_uniqueness_contract', ok: false, detail: parsed.detail };
  const required = Array.isArray(parsed.value?.required_artifacts) ? parsed.value.required_artifacts : [];
  const duplicates = required.filter((row: string, idx: number, arr: string[]) => arr.indexOf(row) !== idx);
  const noncanonical = required.filter(
    (row: string) =>
      typeof row !== 'string'
      || row.trim().length === 0
      || /\s/.test(row)
      || row.includes('..'),
  );
  const violations: string[] = [];
  if (duplicates.length > 0) violations.push(`duplicates=${Array.from(new Set(duplicates)).join(',')}`);
  if (noncanonical.length > 0) violations.push(`noncanonical=${noncanonical.join(',')}`);
  return {
    id: 'release_proof_pack_manifest_required_artifact_uniqueness_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function releaseWorkflowDispatchChannelOptionsContractCheck(): Check {
  const source = read(RELEASE_WORKFLOW_PATH);
  const requiredNeedles = [
    'workflow_dispatch:',
    'release_channel:',
    'default: alpha',
    'type: choice',
    '- alpha',
    '- beta',
    '- stable',
  ];
  const missing = requiredNeedles.filter((needle) => !source.includes(needle));
  return {
    id: 'release_workflow_dispatch_channel_options_contract',
    ok: missing.length === 0,
    detail: missing.length === 0 ? 'ok' : `missing=${missing.join(',')}`,
  };
}

function crossPolicyReleaseContractAlignmentCheck(): Check {
  const channelParsed = parseJson(RELEASE_CHANNEL_POLICY_PATH);
  if (!channelParsed.ok) return { id: 'cross_policy_release_contract_alignment', ok: false, detail: channelParsed.detail };
  const compatibilityParsed = parseJson(RELEASE_COMPATIBILITY_POLICY_PATH);
  if (!compatibilityParsed.ok) return { id: 'cross_policy_release_contract_alignment', ok: false, detail: compatibilityParsed.detail };
  const schemaParsed = parseJson(SCHEMA_VERSIONING_POLICY_PATH);
  if (!schemaParsed.ok) return { id: 'cross_policy_release_contract_alignment', ok: false, detail: schemaParsed.detail };
  const dependencyParsed = parseJson(DEPENDENCY_UPDATE_POLICY_PATH);
  if (!dependencyParsed.ok) return { id: 'cross_policy_release_contract_alignment', ok: false, detail: dependencyParsed.detail };
  const registryParsed = parseJson(API_CLI_REGISTRY_PATH);
  if (!registryParsed.ok) return { id: 'cross_policy_release_contract_alignment', ok: false, detail: registryParsed.detail };
  const compatibility = compatibilityParsed.value || {};
  const registry = registryParsed.value || {};
  const rows = [...(Array.isArray(registry.api_contracts) ? registry.api_contracts : []), ...(Array.isArray(registry.cli_contracts) ? registry.cli_contracts : [])];
  const minimumWindow = Number(compatibility.required_deprecation_days || 0);
  const violations: string[] = [];
  if (compatibility.registry_path !== API_CLI_REGISTRY_PATH) violations.push('registry_path_alignment');
  const channelVersion = asTrimmedString(channelParsed.value?.schema_version);
  const compatibilityVersion = asTrimmedString(compatibilityParsed.value?.schema_version);
  const dependencyVersion = asTrimmedString(dependencyParsed.value?.schema_version);
  const registryVersion = asTrimmedString(registryParsed.value?.schema_version);
  const schemaGateVersion = asTrimmedString(schemaParsed.value?.version);
  const versions = [channelVersion, compatibilityVersion, dependencyVersion, registryVersion, schemaGateVersion];
  if (!versions.every((row) => row === '1.0')) violations.push('version_alignment');
  const hasShortWindow = rows.some((row: any) => Number.isInteger(row?.deprecation_window_days) && row.deprecation_window_days < minimumWindow);
  if (hasShortWindow) violations.push('deprecation_window_alignment');
  return {
    id: 'cross_policy_release_contract_alignment',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function gateRegistryReleaseContractIdsCheck(): Check {
  const parsed = parseJson(TOOLING_GATE_REGISTRY_PATH);
  if (!parsed.ok) return { id: 'gate_registry_release_contract_ids', ok: false, detail: parsed.detail };
  const gates = parsed.value?.gates || {};
  const requiredGateIds = [
    'ops:release-contract:gate',
    'release_policy_gate',
    'ops:production-closure:gate',
    'ops:release:scorecard:gate',
    'ops:release:verdict',
    'ops:runtime-proof:verify',
    'ops:gateway-runtime-chaos:gate',
    'ops:gateway-status:manifest',
    'ops:layer2:parity:guard',
    'ops:layer2:receipt:replay',
    'ops:trusted-core:report',
    'ops:release:proof-pack',
  ];
  const missing = requiredGateIds.filter((id) => typeof gates[id] !== 'object' || gates[id] === null);
  return {
    id: 'gate_registry_release_contract_ids',
    ok: missing.length === 0,
    detail: missing.length === 0 ? 'ok' : `missing=${missing.join(',')}`,
  };
}

function gateRegistryReleaseArtifactBindingsCheck(): Check {
  const parsed = parseJson(TOOLING_GATE_REGISTRY_PATH);
  if (!parsed.ok) return { id: 'gate_registry_release_artifact_bindings', ok: false, detail: parsed.detail };
  const gates = parsed.value?.gates || {};
  const expectedBindings: Array<{ id: string; artifact: string }> = [
    { id: 'ops:release-contract:gate', artifact: 'core/local/artifacts/release_contract_gate_current.json' },
    { id: 'release_policy_gate', artifact: 'core/local/artifacts/release_policy_gate_current.json' },
    { id: 'ops:production-closure:gate', artifact: 'core/local/artifacts/production_readiness_closure_gate_current.json' },
    { id: 'ops:release:verdict', artifact: 'core/local/artifacts/release_verdict_current.json' },
    { id: 'ops:runtime-proof:verify', artifact: 'core/local/artifacts/runtime_proof_verify_current.json' },
    { id: 'ops:gateway-runtime-chaos:gate', artifact: 'core/local/artifacts/gateway_runtime_chaos_gate_current.json' },
    { id: 'ops:gateway-status:manifest', artifact: 'core/local/artifacts/gateway_status_manifest_current.json' },
    { id: 'ops:layer2:parity:guard', artifact: 'core/local/artifacts/layer2_lane_parity_guard_current.json' },
    { id: 'ops:layer2:receipt:replay', artifact: 'core/local/artifacts/layer2_receipt_replay_current.json' },
    { id: 'ops:trusted-core:report', artifact: 'core/local/artifacts/runtime_trusted_core_report_current.json' },
    { id: 'ops:release:proof-pack', artifact: 'core/local/artifacts/release_proof_pack_current.json' },
  ];
  const missingBindings: string[] = [];
  for (const expected of expectedBindings) {
    const row = gates[expected.id];
    const paths = Array.isArray(row?.artifact_paths) ? row.artifact_paths : [];
    if (!paths.includes(expected.artifact)) missingBindings.push(`${expected.id}:${expected.artifact}`);
  }
  return {
    id: 'gate_registry_release_artifact_bindings',
    ok: missingBindings.length === 0,
    detail: missingBindings.length === 0 ? 'ok' : missingBindings.join('; '),
  };
}

function verifyProfilesReleaseCoverageCheck(): Check {
  const parsed = parseJson(VERIFY_PROFILES_PATH);
  if (!parsed.ok) return { id: 'verify_profiles_release_coverage_contract', ok: false, detail: parsed.detail };
  const releaseGateIds = Array.isArray(parsed.value?.profiles?.release?.gate_ids) ? parsed.value.profiles.release.gate_ids : [];
  const required = [
    'ops:release-contract:gate',
    'release_policy_gate',
    'ops:runtime-proof:verify',
    'ops:gateway-runtime-chaos:gate',
    'ops:gateway-status:manifest',
    'ops:layer2:parity:guard',
    'ops:layer2:receipt:replay',
    'ops:trusted-core:report',
    'ops:release:proof-pack',
    'ops:production-closure:gate',
    'ops:release:scorecard:gate',
    'ops:release:verdict',
  ];
  const missing = required.filter((id) => !releaseGateIds.includes(id));
  return {
    id: 'verify_profiles_release_coverage_contract',
    ok: missing.length === 0,
    detail: missing.length === 0 ? 'ok' : `missing=${missing.join(',')}`,
  };
}

function verifyProfilesRuntimeProofCoverageCheck(): Check {
  const parsed = parseJson(VERIFY_PROFILES_PATH);
  if (!parsed.ok) return { id: 'verify_profiles_runtime_proof_coverage_contract', ok: false, detail: parsed.detail };
  const runtimeGateIds = Array.isArray(parsed.value?.profiles?.['runtime-proof']?.gate_ids)
    ? parsed.value.profiles['runtime-proof'].gate_ids
    : [];
  const required = [
    'ops:runtime-proof:verify',
    'ops:gateway-runtime-chaos:gate',
    'ops:layer2:parity:guard',
    'ops:layer2:receipt:replay',
    'ops:boundedness:release-gate',
    'ops:trusted-core:report',
  ];
  const missing = required.filter((id) => !runtimeGateIds.includes(id));
  return {
    id: 'verify_profiles_runtime_proof_coverage_contract',
    ok: missing.length === 0,
    detail: missing.length === 0 ? 'ok' : `missing=${missing.join(',')}`,
  };
}

function releaseWorkflowContractStepsCheck(): Check {
  const source = read(RELEASE_WORKFLOW_PATH);
  const requiredNeedles = [
    'Release Runtime Contract Gate',
    'SRS Full Regression Gate (Strict)',
    'npm run -s ops:runtime-proof:verify',
    'npm run -s ops:workspace-tooling:release-proof',
    'npm run -s ops:layer2:parity:guard',
    'npm run -s ops:layer2:receipt:replay',
    'npm run -s ops:gateway-status:manifest',
    'npm run -s ops:trusted-core:report',
    'npm run -s ops:release:proof-pack -- --version=${{ steps.semver.outputs.tag }}',
    'npm run -s ops:production-closure:gate',
    'npm run -s ops:release:verdict',
  ];
  const missing = requiredNeedles.filter((needle) => !source.includes(needle));
  return {
    id: 'release_workflow_contract_steps',
    ok: missing.length === 0,
    detail: missing.length === 0 ? 'ok' : `missing=${missing.join(',')}`,
  };
}

function releaseWorkflowProofPackEnforcementCheck(): Check {
  const source = read(RELEASE_WORKFLOW_PATH);
  const requiredNeedles = [
    'Enforce mandatory release proof-pack artifacts',
    'required_missing',
    'category_threshold_failure_count',
    'layer2_lane_parity_guard_current.json',
    'layer2_receipt_replay_current.json',
    'runtime_trusted_core_report_current.json',
    'release_proof_pack_contract_failed',
  ];
  const missing = requiredNeedles.filter((needle) => !source.includes(needle));
  return {
    id: 'release_workflow_proof_pack_enforcement_contract',
    ok: missing.length === 0,
    detail: missing.length === 0 ? 'ok' : `missing=${missing.join(',')}`,
  };
}

function releaseProofPackManifestSchemaCheck(): Check {
  const parsed = parseJson(RELEASE_PROOF_PACK_MANIFEST_PATH);
  if (!parsed.ok) return { id: 'release_proof_pack_manifest_schema_contract', ok: false, detail: parsed.detail };
  const manifest = parsed.value || {};
  const groups = manifest.artifact_groups || {};
  const completeness = manifest.category_completeness_min || {};
  const violations: string[] = [];
  if (manifest.version !== 1) violations.push('version');
  const requiredGroups = ['runtime_proof', 'adapter_and_orchestration', 'release_governance', 'workload_and_quality'];
  for (const group of requiredGroups) {
    if (!Array.isArray(groups[group])) violations.push(`artifact_groups:${group}`);
    if (completeness[group] !== 1) violations.push(`category_completeness_min:${group}`);
  }
  if (!Array.isArray(manifest.required_artifacts) || manifest.required_artifacts.length === 0) {
    violations.push('required_artifacts');
  }
  return {
    id: 'release_proof_pack_manifest_schema_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function releaseProofPackManifestRequiredArtifactsCheck(): Check {
  const parsed = parseJson(RELEASE_PROOF_PACK_MANIFEST_PATH);
  if (!parsed.ok) return { id: 'release_proof_pack_manifest_required_artifacts_contract', ok: false, detail: parsed.detail };
  const required = Array.isArray(parsed.value?.required_artifacts) ? parsed.value.required_artifacts : [];
  const mustHave = [
    'core/local/artifacts/layer2_lane_parity_guard_current.json',
    'core/local/artifacts/layer2_receipt_replay_current.json',
    'core/local/artifacts/runtime_trusted_core_report_current.json',
    'core/local/artifacts/release_proof_pack_current.json',
    'core/local/artifacts/runtime_proof_verify_current.json',
    'core/local/artifacts/gateway_status_manifest_current.json',
    'core/local/artifacts/gateway_runtime_chaos_gate_current.json',
    'core/local/artifacts/production_readiness_closure_gate_current.json',
    'core/local/artifacts/shell_truth_leak_guard_current.json',
    'core/local/artifacts/windows_installer_contract_guard_current.json',
  ];
  const missing = mustHave.filter((artifact) => !required.includes(artifact));
  return {
    id: 'release_proof_pack_manifest_required_artifacts_contract',
    ok: missing.length === 0,
    detail: missing.length === 0 ? 'ok' : `missing=${missing.join(',')}`,
  };
}

function releaseProofPackManifestCategoryMembershipCheck(): Check {
  const parsed = parseJson(RELEASE_PROOF_PACK_MANIFEST_PATH);
  if (!parsed.ok) return { id: 'release_proof_pack_manifest_category_membership_contract', ok: false, detail: parsed.detail };
  const groups = parsed.value?.artifact_groups || {};
  const runtimeProof = Array.isArray(groups.runtime_proof) ? groups.runtime_proof : [];
  const adapterAndOrchestration = Array.isArray(groups.adapter_and_orchestration) ? groups.adapter_and_orchestration : [];
  const releaseGovernance = Array.isArray(groups.release_governance) ? groups.release_governance : [];
  const workloadAndQuality = Array.isArray(groups.workload_and_quality) ? groups.workload_and_quality : [];
  const violations: string[] = [];
  if (!runtimeProof.includes('core/local/artifacts/runtime_proof_verify_current.json')) violations.push('runtime_proof:runtime_proof_verify');
  if (!adapterAndOrchestration.includes('core/local/artifacts/gateway_status_manifest_current.json')) {
    violations.push('adapter_and_orchestration:gateway_status_manifest');
  }
  if (!adapterAndOrchestration.includes('core/local/artifacts/layer2_lane_parity_guard_current.json')) {
    violations.push('adapter_and_orchestration:layer2_lane_parity_guard');
  }
  if (!releaseGovernance.includes('core/local/artifacts/runtime_trusted_core_report_current.json')) {
    violations.push('release_governance:runtime_trusted_core_report');
  }
  if (!workloadAndQuality.includes('core/local/artifacts/workspace_tooling_release_proof_current.json')) {
    violations.push('workload_and_quality:workspace_tooling_release_proof');
  }
  return {
    id: 'release_proof_pack_manifest_category_membership_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function releaseFlowCrossContractAlignmentCheck(): Check {
  const registryParsed = parseJson(TOOLING_GATE_REGISTRY_PATH);
  if (!registryParsed.ok) return { id: 'release_flow_cross_contract_alignment', ok: false, detail: registryParsed.detail };
  const profileParsed = parseJson(VERIFY_PROFILES_PATH);
  if (!profileParsed.ok) return { id: 'release_flow_cross_contract_alignment', ok: false, detail: profileParsed.detail };
  const manifestParsed = parseJson(RELEASE_PROOF_PACK_MANIFEST_PATH);
  if (!manifestParsed.ok) return { id: 'release_flow_cross_contract_alignment', ok: false, detail: manifestParsed.detail };
  const registryGates = registryParsed.value?.gates || {};
  const releaseProfileGateIds = Array.isArray(profileParsed.value?.profiles?.release?.gate_ids)
    ? profileParsed.value.profiles.release.gate_ids
    : [];
  const requiredArtifacts = Array.isArray(manifestParsed.value?.required_artifacts) ? manifestParsed.value.required_artifacts : [];
  const alignmentChecks = [
    { gate: 'ops:layer2:parity:guard', artifact: 'core/local/artifacts/layer2_lane_parity_guard_current.json' },
    { gate: 'ops:layer2:receipt:replay', artifact: 'core/local/artifacts/layer2_receipt_replay_current.json' },
    { gate: 'ops:trusted-core:report', artifact: 'core/local/artifacts/runtime_trusted_core_report_current.json' },
    { gate: 'ops:gateway-status:manifest', artifact: 'core/local/artifacts/gateway_status_manifest_current.json' },
    { gate: 'ops:gateway-runtime-chaos:gate', artifact: 'core/local/artifacts/gateway_runtime_chaos_gate_current.json' },
    { gate: 'ops:runtime-proof:verify', artifact: 'core/local/artifacts/runtime_proof_verify_current.json' },
  ];
  const violations: string[] = [];
  for (const row of alignmentChecks) {
    const gateRow = registryGates[row.gate];
    const gateArtifacts = Array.isArray(gateRow?.artifact_paths) ? gateRow.artifact_paths : [];
    if (!releaseProfileGateIds.includes(row.gate)) violations.push(`${row.gate}:missing_from_release_profile`);
    if (!gateArtifacts.includes(row.artifact)) violations.push(`${row.gate}:missing_registry_artifact`);
    if (!requiredArtifacts.includes(row.artifact)) violations.push(`${row.gate}:missing_manifest_required_artifact`);
  }
  return {
    id: 'release_flow_cross_contract_alignment',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function wrapperContractCheck(): Check {
  const violations: string[] = [];
  for (const rel of WRAPPER_FILES) {
    const source = read(rel);
    const hasBootstrapEntrypoint =
      source.includes('ts_bootstrap.ts') && source.includes('bootstrap(__filename, module)');
    const hasRustLaneBridge = source.includes('createOpsLaneBridge');
    const hasSurfaceShim =
      source.includes('surface/orchestration/scripts/') && source.includes('thin CLI bridge');
    if (!(hasBootstrapEntrypoint || hasRustLaneBridge || hasSurfaceShim)) violations.push(`${rel}:missing_contract`);
    if (source.includes('legacy_retired_lane_bridge')) violations.push(`${rel}:legacy_retired_lane_bridge`);
  }
  return {
    id: 'conduit_wrapper_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? `checked=${WRAPPER_FILES.length}` : violations.join('; '),
  };
}

function installerContractCheck(): Check {
  const source = read('install.sh');
  const ok =
    source.includes('api.github.com/repos') &&
    source.includes('protheus-ops') &&
    source.includes('infringd') &&
    source.includes('--repair') &&
    source.includes('verify_workspace_runtime_contract') &&
    source.includes('run_post_install_smoke_tests') &&
    source.includes('dashboard_route_check');
  return {
    id: 'installer_contract',
    ok,
    detail: ok ? 'ok' : 'install.sh missing hosted-installer or runtime-integrity markers',
  };
}

function windowsAndDocsCheck(): Check {
  const installPs = read('install.ps1');
  const opsLib = read('core/layer0/ops/src/lib.rs');
  const readme = read('README.md');
  const gettingStarted = read('docs/client/GETTING_STARTED.md');
  const manualHelp = read('docs/workspace/manuals/infring_manual_help_tab.md');
  const installPsForceRepairShim = /if \(\$Force\)\s*\{[\s\S]*\$InstallRepair\s*=\s*\$true[\s\S]*if \(-not \$Minimal\)\s*\{[\s\S]*\$InstallFull\s*=\s*\$true/.test(
    installPs,
  );
  const windowsBuildToolsCommand =
    'winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools"';
  const directBootstrapperUrl = 'https://aka.ms/vs/17/release/vs_BuildTools.exe';
  const directGatewayFallbackCommand = '$HOME\\.infring\\bin\\infring.cmd gateway';
  const noFileFallbackIex = 'irm https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.ps1 | iex';
  const executionPolicyBypassForce = 'Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force';
  const rerunReadmeInstallHint = 'rerun the README Windows install command: $ReadmeWindowsInstallCommand';
  const readmeCommandBanner = '[infring install] README Windows install command: $ReadmeWindowsInstallCommand';
  const preflightToolchainBanner =
    '[infring install] preflight windows toolchain: cargo={0}; rustc={1}; msvc_tools={2}; tar={3}; winget={4}';
  const preflightTripleCandidatesBanner = '[infring install] preflight triple candidates: {0}';
  const preflightAssetFoundProbeBanner =
    '[infring install] preflight asset probe ({0}): found {1}; reachable={2} ({3})';
  const preflightAssetMissingProbeBanner =
    '[infring install] preflight asset probe ({0}): missing prebuilt in release metadata ({1})';
  const preflightPolicyBanner =
    '[infring install] preflight policy: allow_no_msvc_source_fallback={0}; compatible_release_fallback={1}; pinned_version_compatible_fallback={2}';
  const preflightCompatibleTripleNoteBanner =
    '[infring install] preflight note: using compatible Windows triple asset variant {0} for requested {1}';
  const preflightMsvcMissingWarning =
    '[infring install] preflight warning: MSVC build tools were not detected; source fallback may fail if Windows prebuilt assets are unavailable.';
  const preflightMsvcBootstrapEnabledNote =
    '[infring install] preflight note: auto MSVC bootstrap is enabled (INFRING_INSTALL_AUTO_MSVC=1 default); installer will attempt winget bootstrap first and direct bootstrapper fallback if needed.';
  const preflightWingetUnavailableDirectEnabledNote =
    '[infring install] preflight note: winget is unavailable; installer will attempt direct Build Tools bootstrapper download during source fallback.';
  const preflightWingetUnavailableDirectDisabledWarning =
    '[infring install] preflight warning: winget is unavailable and direct bootstrap fallback is disabled; install Build Tools manually.';
  const preflightAutoMsvcDisabledNote =
    '[infring install] preflight note: auto MSVC bootstrap is disabled (set INFRING_INSTALL_AUTO_MSVC=1 to enable automatic Build Tools install attempts).';
  const preflightTarMissingWarning =
    '[infring install] preflight warning: tar was not detected; archive prebuilt extraction and some source fallback paths may fail.';
  const preflightLatestAssetGapWarning =
    '[infring install] preflight warning: current latest tag has Windows asset gaps and source fallback prerequisites are limited; installer will still try compatible-tag fallback before failing.';
  const preflightCargoAutoRustupNote =
    '[infring install] preflight note: Cargo missing but auto Rust bootstrap is enabled; installer will attempt toolchain bootstrap during source fallback.';
  const preflightCargoAutoRustupDisabledThrow =
    'Windows installer preflight failed: prebuilt asset gaps detected for [$gapSummary], Cargo is unavailable, and auto Rust bootstrap is disabled (INFRING_INSTALL_AUTO_RUSTUP=0 or INFRING_AUTO_RUSTUP=0). Install Rust + MSVC build tools or publish missing Windows release assets.';
  const preflightNoReachablePrebuiltMsvcMissingNote =
    '[infring install] preflight note: no reachable Windows prebuilt and MSVC tools missing; attempting best-effort source fallback';
  const preflightNoReachablePrebuiltMsvcMissingForcedNote =
    '[infring install] preflight note: no reachable Windows prebuilt + MSVC tools missing; forcing best-effort source fallback despite INFRING_INSTALL_ALLOW_NO_MSVC_SOURCE_FALLBACK=0';
  const preflightRecommendedBuildToolsFix =
    '[infring install] recommended fix: winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools"';
  const pureInstallFailureThrowPrefix =
    'Failed to install pure workspace binary for $triple ($resolvedVersionLabel). No compatible prebuilt asset was found and source fallback did not complete. Diagnostic: $failureHint';
  const coreInstallFailureThrowPrefix =
    'Failed to install core ops runtime for $triple ($resolvedVersionLabel). Prebuilt asset download failed and source fallback did not complete. Diagnostic: $failureHint';
  const windowsFailureRemediationSentence =
    'Install Rust toolchain + C++ build tools, then rerun the README Windows install command: $ReadmeWindowsInstallCommand $windowsToolsHint';
  const noCompatiblePrebuiltBanner =
    '[infring install] no compatible Windows prebuilt release found for required stems; source fallback remains a backup path only.';
  const compatibleReleaseFallbackDisabledBanner =
    '[infring install] compatible Windows release fallback is disabled (set INFRING_INSTALL_ALLOW_COMPATIBLE_RELEASE_FALLBACK=1 to enable alternate-tag prebuilt scanning).';
  const pinnedCompatibleReleaseFallbackBanner =
    '[infring install] pinned release $version is missing one or more required Windows prebuilts for $triple; using compatible release $compatibleWindows (disable with INFRING_INSTALL_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK=0)';
  const pinnedCompatibleFallbackDisabledNote =
    '[infring install] pinned Windows compatible-release fallback is disabled; set INFRING_INSTALL_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK=1 to allow compatible prebuilt selection when pinned tag assets are unavailable.';
  const sourceFallbackPolicyBanner =
    '[infring install] source fallback policy: main_last_resort_fallback={0}';
  const sourceFallbackAppendMainRetryBanner =
    '[infring install] source fallback for {0} failed ({1}); appending main as last-resort source retry';
  const sourceFallbackReleaseRetryFromMainBanner =
    '[infring install] source fallback for release $Version failed ($script:LastBinaryInstallFailureReason); retrying from main branch';
  const sourceFallbackMainFirstBanner =
    '[infring install] source fallback using main first (missing prebuilt asset metadata for $Stem on $Triple)';
  const sourceFallbackPlanBanner = '[infring install] source fallback plan: {0}';
  const autoMsvcEnabledBanner =
    '[infring install] auto MSVC bootstrap is enabled; installer will attempt Build Tools install during source fallback if needed.';
  const autoMsvcDisabledBanner =
    '[infring install] auto MSVC bootstrap is disabled; enable with INFRING_INSTALL_AUTO_MSVC=1 for best-effort source fallback repair.';
  const windowsBuildToolsHintWinget =
    'Install Visual Studio Build Tools (MSVC+C++) via winget: winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override ""--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools""';
  const windowsBuildToolsHintNoWinget =
    'fallback (no winget): `$vs = Join-Path `$env:TEMP ""vs_BuildTools.exe""; irm https://aka.ms/vs/17/release/vs_BuildTools.exe -OutFile `$vs; Start-Process -FilePath `$vs -ArgumentList ""--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"" -Wait';
  const failureHintRequiredTokens = [
    'asset_probe={0};reachable={1};status={2}',
    'asset_probe=missing;status={0}',
    'asset_probe_triple={0}',
    'asset_probe_triple_candidates={0}',
    'attempted_assets={0}',
    'source_fallback_attempted={0}',
    'source_fallback_versions={0}',
    'source_fallback_reason={0}',
    'preflight_no_reachable_prebuilt_with_missing_msvc={0}',
    'source_fallback_plan={0}',
    'auto_msvc_bootstrap_enabled={0}',
    'main_last_resort_fallback={0}',
    'toolchain:cargo={0};rustc={1};msvc_tools={2};tar={3};winget={4}',
    'auto_bootstrap:auto_rustup={0};auto_msvc={1}',
    'auto_msvc=',
    'auto_bootstrap:direct_msvc={0}',
    'install_policy:allow_no_msvc_source_fallback={0};compatible_release_fallback={1};pinned_version_compatible_fallback={2}',
  ];
  const failureReasonTaxonomyTokens = [
    'cargo_missing',
    'cargo_missing_auto_rustup_disabled',
    'rustup_bootstrap_failed',
    'source_repo_unavailable',
    'msvc_tools_missing_no_reachable_prebuilt_asset',
    'msvc_tools_missing_auto_bootstrap_disabled',
    'msvc_bootstrap_winget_unavailable',
    'msvc_bootstrap_direct_disabled',
    'msvc_tools_still_missing_after_bootstrap',
    'source_build_output_missing',
    'asset_archive_extract_failed',
  ];
  const windowsReadmeInstallCommand =
    'Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force; $tmp = Join-Path $env:TEMP "infring-install.ps1"; irm https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.ps1 -OutFile $tmp -ErrorAction Stop; & $tmp -Repair -Full; Remove-Item $tmp -Force -ErrorAction SilentlyContinue';
  const hasFailureHintTokenCoverage = failureHintRequiredTokens.every((token) => installPs.includes(token));
  const hasFailureReasonTaxonomyCoverage = failureReasonTaxonomyTokens.every((token) => installPs.includes(token));
  const ok =
    installPs.includes('protheus-ops.exe') &&
    installPs.includes('infringd.cmd') &&
    installPs.includes('Install-AllowNoMsvcSourceFallback') &&
    installPs.includes('INFRING_INSTALL_ALLOW_NO_MSVC_SOURCE_FALLBACK') &&
    installPs.includes('INFRING_ALLOW_NO_MSVC_SOURCE_FALLBACK') &&
    installPs.includes('Install-AllowCompatibleReleaseFallback') &&
    installPs.includes('Install-AllowPinnedVersionCompatibleFallback') &&
    installPs.includes('INFRING_ALLOW_COMPATIBLE_RELEASE_FALLBACK') &&
    installPs.includes('INFRING_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK') &&
    installPs.includes('INFRING_INSTALL_AUTO_RUSTUP') &&
    installPs.includes('INFRING_AUTO_RUSTUP') &&
    installPs.includes('INFRING_INSTALL_AUTO_MSVC') &&
    installPs.includes('INFRING_AUTO_MSVC_BOOTSTRAP') &&
    installPs.includes('INFRING_AUTO_MSVC') &&
    installPs.includes('INFRING_INSTALL_ALLOW_DIRECT_MSVC_BOOTSTRAP') &&
    installPs.includes('INFRING_ALLOW_DIRECT_MSVC_BOOTSTRAP') &&
    installPs.includes('INFRING_INSTALL_ALLOW_MAIN_LAST_RESORT_SOURCE_FALLBACK') &&
    installPs.includes('INFRING_ALLOW_MAIN_LAST_RESORT_SOURCE_FALLBACK') &&
    installPs.includes('Install-AllowDirectMsvcBootstrapEnabled') &&
    installPs.includes('INFRING_INSTALL_REPAIR') &&
    installPs.includes('INFRING_INSTALL_FULL') &&
    installPs.includes('Compatibility shim for operators accustomed to `-Force`.') &&
    installPsForceRepairShim &&
    installPs.includes(rerunReadmeInstallHint) &&
    installPs.includes(readmeCommandBanner) &&
    installPs.includes(preflightToolchainBanner) &&
    installPs.includes(preflightTripleCandidatesBanner) &&
    installPs.includes(preflightAssetFoundProbeBanner) &&
    installPs.includes(preflightAssetMissingProbeBanner) &&
    installPs.includes(preflightPolicyBanner) &&
    installPs.includes(preflightCompatibleTripleNoteBanner) &&
    installPs.includes(preflightMsvcMissingWarning) &&
    installPs.includes(preflightMsvcBootstrapEnabledNote) &&
    installPs.includes(preflightWingetUnavailableDirectEnabledNote) &&
    installPs.includes(preflightWingetUnavailableDirectDisabledWarning) &&
    installPs.includes(preflightAutoMsvcDisabledNote) &&
    installPs.includes(preflightTarMissingWarning) &&
    installPs.includes(preflightLatestAssetGapWarning) &&
    installPs.includes(preflightCargoAutoRustupNote) &&
    installPs.includes(preflightCargoAutoRustupDisabledThrow) &&
    installPs.includes(preflightNoReachablePrebuiltMsvcMissingNote) &&
    installPs.includes(preflightNoReachablePrebuiltMsvcMissingForcedNote) &&
    installPs.includes(preflightRecommendedBuildToolsFix) &&
    installPs.includes(pureInstallFailureThrowPrefix) &&
    installPs.includes(coreInstallFailureThrowPrefix) &&
    installPs.includes(windowsFailureRemediationSentence) &&
    installPs.includes(noCompatiblePrebuiltBanner) &&
    installPs.includes(compatibleReleaseFallbackDisabledBanner) &&
    installPs.includes(pinnedCompatibleReleaseFallbackBanner) &&
    installPs.includes(pinnedCompatibleFallbackDisabledNote) &&
    installPs.includes(sourceFallbackPolicyBanner) &&
    installPs.includes(sourceFallbackAppendMainRetryBanner) &&
    installPs.includes(sourceFallbackReleaseRetryFromMainBanner) &&
    installPs.includes(sourceFallbackMainFirstBanner) &&
    installPs.includes(sourceFallbackPlanBanner) &&
    installPs.includes(autoMsvcEnabledBanner) &&
    installPs.includes(autoMsvcDisabledBanner) &&
    installPs.includes(windowsBuildToolsHintWinget) &&
    installPs.includes(windowsBuildToolsHintNoWinget) &&
    hasFailureHintTokenCoverage &&
    hasFailureReasonTaxonomyCoverage &&
    opsLib.includes('#![recursion_limit = "16384"]') &&
    installPs.includes(directBootstrapperUrl) &&
    installPs.includes(windowsReadmeInstallCommand) &&
    /& \$tmp(?:\s+-Repair)?\s+-Full/.test(readme) &&
    readme.includes('install.ps1 -OutFile $tmp -ErrorAction Stop') &&
    readme.includes(executionPolicyBypassForce) &&
    readme.includes('Remove-Item $tmp -Force -ErrorAction SilentlyContinue') &&
    readme.includes(windowsBuildToolsCommand) &&
    readme.includes(directBootstrapperUrl) &&
    readme.includes('$env:INFRING_INSTALL_AUTO_MSVC = "0"') &&
    readme.includes('$env:INFRING_INSTALL_ALLOW_DIRECT_MSVC_BOOTSTRAP = "0"') &&
    readme.includes('$env:INFRING_INSTALL_AUTO_RUSTUP = "0"') &&
    readme.includes('$env:INFRING_INSTALL_ALLOW_COMPATIBLE_RELEASE_FALLBACK = "0"') &&
    readme.includes('$env:INFRING_INSTALL_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK = "0"') &&
    readme.includes('$env:INFRING_INSTALL_REPAIR = "1"') &&
    readme.includes('$env:INFRING_INSTALL_FULL = "1"') &&
    readme.includes(noFileFallbackIex) &&
    !readme.includes('| iex -Full') &&
    readme.includes(directGatewayFallbackCommand) &&
    readme.includes('INFRING_INSTALL_ALLOW_DIRECT_MSVC_BOOTSTRAP') &&
    /& \$tmp(?:\s+-Repair)?\s+-Full/.test(gettingStarted) &&
    gettingStarted.includes('install.ps1 -OutFile $tmp -ErrorAction Stop') &&
    gettingStarted.includes(executionPolicyBypassForce) &&
    gettingStarted.includes('Remove-Item $tmp -Force -ErrorAction SilentlyContinue') &&
    gettingStarted.includes(windowsBuildToolsCommand) &&
    gettingStarted.includes('$env:INFRING_INSTALL_AUTO_MSVC = "0"') &&
    gettingStarted.includes('$env:INFRING_INSTALL_ALLOW_DIRECT_MSVC_BOOTSTRAP = "0"') &&
    gettingStarted.includes('$env:INFRING_INSTALL_AUTO_RUSTUP = "0"') &&
    gettingStarted.includes('$env:INFRING_INSTALL_ALLOW_COMPATIBLE_RELEASE_FALLBACK = "0"') &&
    gettingStarted.includes('$env:INFRING_INSTALL_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK = "0"') &&
    gettingStarted.includes('$env:INFRING_INSTALL_REPAIR = "1"') &&
    gettingStarted.includes('$env:INFRING_INSTALL_FULL = "1"') &&
    gettingStarted.includes(noFileFallbackIex) &&
    !gettingStarted.includes('| iex -Full') &&
    gettingStarted.includes(directGatewayFallbackCommand) &&
    gettingStarted.includes('INFRING_INSTALL_ALLOW_DIRECT_MSVC_BOOTSTRAP') &&
    gettingStarted.includes('infring --help') &&
    manualHelp.includes('install.ps1 -OutFile $tmp -ErrorAction Stop') &&
    manualHelp.includes(executionPolicyBypassForce) &&
    manualHelp.includes('Remove-Item $tmp -Force -ErrorAction SilentlyContinue') &&
    manualHelp.includes(windowsBuildToolsCommand) &&
    manualHelp.includes(directBootstrapperUrl) &&
    manualHelp.includes('$env:INFRING_INSTALL_AUTO_MSVC = "0"') &&
    manualHelp.includes('$env:INFRING_INSTALL_ALLOW_DIRECT_MSVC_BOOTSTRAP = "0"') &&
    manualHelp.includes('$env:INFRING_INSTALL_AUTO_RUSTUP = "0"') &&
    manualHelp.includes('$env:INFRING_INSTALL_ALLOW_COMPATIBLE_RELEASE_FALLBACK = "0"') &&
    manualHelp.includes('$env:INFRING_INSTALL_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK = "0"') &&
    manualHelp.includes('$env:INFRING_INSTALL_REPAIR = "1"') &&
    manualHelp.includes('$env:INFRING_INSTALL_FULL = "1"') &&
    manualHelp.includes(noFileFallbackIex) &&
    !manualHelp.includes('| iex -Full') &&
    manualHelp.includes(directGatewayFallbackCommand) &&
    manualHelp.includes('INFRING_INSTALL_ALLOW_DIRECT_MSVC_BOOTSTRAP') &&
    /& \$tmp(?:\s+-Repair)?\s+-Full/.test(manualHelp);
  return {
    id: 'windows_and_docs_contract',
    ok,
    detail: ok ? 'ok' : 'windows installer or getting started contract drifted',
  };
}

function architectureDocsCheck(): Check {
  const architecture = read('ARCHITECTURE.md');
  return {
    id: 'architecture_docs_contract',
    ok: architecture.includes('```mermaid') && architecture.includes('Conduit') && architecture.includes('Core'),
    detail: 'ARCHITECTURE.md must retain conduit mermaid map',
  };
}

function transportLockCheck(): Check {
  const sdk = read('packages/infring-sdk/src/transports.ts');
  const sdkCliDevOnly = read('packages/infring-sdk/src/transports/cli_dev_only.ts');
  const bridge = read('adapters/runtime/ops_lane_bridge.ts');
  const runner = read('adapters/runtime/run_protheus_ops.ts');
  const ok =
    sdk.includes('resident_ipc_authoritative') &&
    sdk.includes('createResidentIpcTransport') &&
    !sdk.includes("node:child_process") &&
    sdkCliDevOnly.includes('process_transport_forbidden_in_production') &&
    sdkCliDevOnly.includes('isProductionReleaseChannel') &&
    bridge.includes('process_fallback_forbidden_in_production') &&
    bridge.includes('processFallbackPolicy') &&
    runner.includes('createOpsLaneBridge') &&
    runner.includes('INFRING_OPS_FORCE_LEGACY_PROCESS_RUNNER');
  return {
    id: 'transport_lock_contract',
    ok,
    detail: ok ? 'ok' : 'sdk/bridge/runner production transport lock markers missing',
  };
}

function collectTopologyStatusViaEntrypoint(envOverrides: NodeJS.ProcessEnv): any {
  const stdout = execFileSync('node', [TS_ENTRYPOINT, TOPOLOGY_STATUS_SCRIPT, '--json=1'], {
    cwd: ROOT,
    encoding: 'utf8',
    env: {
      ...process.env,
      ...envOverrides,
    },
  });
  return JSON.parse(String(stdout || '{}').trim() || '{}');
}

function topologyModeChecks(): Check[] {
  const dev = collectTopologyStatusViaEntrypoint({
    ...process.env,
    INFRING_RELEASE_CHANNEL: 'dev',
    INFRING_OPS_ALLOW_PROCESS_FALLBACK: '1',
  });
  const stable = collectTopologyStatusViaEntrypoint({
    ...process.env,
    INFRING_RELEASE_CHANNEL: 'stable',
    INFRING_OPS_IPC_DAEMON: '1',
    INFRING_OPS_IPC_STRICT: '1',
    INFRING_OPS_ALLOW_PROCESS_FALLBACK: '0',
    INFRING_SDK_ALLOW_PROCESS_TRANSPORT: '0',
  });
  const devShapeOk =
    !!dev
    && typeof dev === 'object'
    && typeof dev.ok === 'boolean'
    && Array.isArray(dev.violations)
    && !!dev.transport
    && typeof dev.transport === 'object'
    && typeof dev.transport.process_fallback_effective === 'boolean';
  const stableShapeOk =
    !!stable
    && typeof stable === 'object'
    && typeof stable.ok === 'boolean'
    && Array.isArray(stable.violations)
    && !!stable.transport
    && typeof stable.transport === 'object'
    && typeof stable.transport.process_fallback_effective === 'boolean';
  return [
    {
      id: 'transport_topology_payload_shape_contract',
      ok: devShapeOk && stableShapeOk,
      detail: `dev_shape=${String(devShapeOk)};stable_shape=${String(stableShapeOk)}`,
    },
    {
      id: 'transport_topology_dev_guard',
      ok: dev.ok === false && dev.violations.some((row: any) => row.id === 'ops_process_fallback_effective'),
      detail: 'dev fallback should degrade topology',
    },
    {
      id: 'transport_topology_stable_guard',
      ok: stable.ok === true && stable.production_release === true && stable.transport.process_fallback_effective === false,
      detail: 'stable topology should remain resident-ipc-only',
    },
  ];
}

function buildReport(args: ReturnType<typeof parseArgs>) {
  const checks: Check[] = [
    ...releaseContractPathAndConstantChecks(args),
    ...releaseContractFilePresenceChecks(),
    ...releaseContractWrapperListChecks(),
    runGateCheck('runtime_dependency_contract'),
    runGateCheck('ops:legacy-runner:release-guard'),
    runGateCheck('ops:transport:spawn-audit'),
    runGateCheck('release_policy_gate'),
    runGateCheck('ops:assimilation:v1:support:guard'),
    releaseChannelPolicySchemaContractCheck(),
    releaseChannelPromotionContractCheck(),
    releaseChannelPolicyKeysetContractCheck(),
    releaseChannelPolicyChannelTokenFormatContractCheck(),
    releaseChannelPromotionRuleShapeContractCheck(),
    releaseCompatibilityPolicyContractCheck(),
    releaseCompatibilityPolicyKeysetContractCheck(),
    releaseCompatibilityRegistryPathFormatContractCheck(),
    apiCliContractRegistrySchemaContractCheck(),
    apiCliRegistryContractVersionSemverContractCheck(),
    apiCliRegistryContractNameTokenContractCheck(),
    apiCliContractRegistryDeprecationContractCheck(),
    schemaVersioningGatePolicyContractCheck(),
    schemaVersioningTargetIdTokenContractCheck(),
    schemaVersioningTargetPathContractCheck(),
    schemaVersioningTargetSchemaIdAlignmentContractCheck(),
    schemaVersioningTargetVersionSemverContractCheck(),
    schemaVersioningOutputsKeysetContractCheck(),
    schemaVersioningGateOutputsContractCheck(),
    dependencyUpdatePolicyContractCheck(),
    dependencyUpdatePolicyKeysetContractCheck(),
    dependencyUpdateBlocklistContractCheck(),
    dependencyUpdateBlockedPackageNameTokenContractCheck(),
    dependencyUpdateBlockedPackageReasonQualityContractCheck(),
    crossPolicyReleaseContractAlignmentCheck(),
    gateRegistryReleaseContractIdsCheck(),
    gateRegistryReleaseArtifactBindingsCheck(),
    verifyProfilesReleaseCoverageCheck(),
    verifyProfilesReleaseGateUniquenessContractCheck(),
    verifyProfilesRuntimeProofCoverageCheck(),
    verifyProfilesRuntimeProofGateUniquenessContractCheck(),
    releaseWorkflowContractStepsCheck(),
    releaseWorkflowDispatchChannelOptionsContractCheck(),
    releaseWorkflowProofPackEnforcementCheck(),
    releaseProofPackManifestSchemaCheck(),
    releaseProofPackManifestCategoryKeysetContractCheck(),
    releaseProofPackManifestRequiredArtifactsCheck(),
    releaseProofPackManifestRequiredArtifactUniquenessContractCheck(),
    releaseProofPackManifestCategoryMembershipCheck(),
    releaseFlowCrossContractAlignmentCheck(),
    wrapperContractCheck(),
    installerContractCheck(),
    windowsAndDocsCheck(),
    architectureDocsCheck(),
    transportLockCheck(),
    ...topologyModeChecks(),
  ];
  const failed = checks.filter((row) => !row.ok);
  return {
    ok: failed.length === 0,
    type: 'release_contract_gate',
    generated_at: new Date().toISOString(),
    summary: {
      check_count: checks.length,
      failed_count: failed.length,
    },
    failed_ids: failed.map((row) => row.id),
    checks,
  };
}

function run(argv: string[] = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const report = buildReport(args);
  const outPath = path.resolve(ROOT, args.out);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  if (args.strict && report.ok !== true) return 1;
  return 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  buildReport,
  run,
};
