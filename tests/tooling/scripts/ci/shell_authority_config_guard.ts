#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');
const childProcess = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');

type Check = {
  id: string;
  ok: boolean;
  detail: string;
};

const MIRROR_CONFIGS = [
  ['client/runtime/config/state_tier_manifest.json', 'core/layer0/ops/config/state_tier_manifest.json', 'core/layer0/ops'],
  ['client/runtime/config/egress_gateway_policy.json', 'core/layer0/ops/config/egress_gateway_policy.json', 'core/layer0/ops'],
  ['client/runtime/config/batch_query_policy.json', 'core/layer0/ops/config/batch_query_policy.json', 'core/layer0/ops'],
  ['client/runtime/config/web_conduit_policy.json', 'core/layer0/ops/config/web_conduit_policy.json', 'core/layer0/ops'],
  ['client/runtime/config/provider_network_policy.json', 'core/layer0/ops/config/provider_network_policy.json', 'core/layer0/ops'],
  ['client/runtime/config/secret_broker_policy.json', 'core/layer0/ops/config/secret_broker_policy.json', 'core/layer0/ops'],
  ['client/runtime/config/rust_source_of_truth_policy.json', 'core/layer0/ops/config/rust_source_of_truth_policy.json', 'core/layer0/ops'],
  ['client/runtime/config/kernel_naming_policy.json', 'core/layer0/ops/config/kernel_naming_policy.json', 'core/layer0/ops'],
  ['client/runtime/config/abac_policy_plane.json', 'core/layer1/security/config/abac_policy_plane.json', 'core/layer1/security'],
  ['client/runtime/config/security_layer_inventory.json', 'validation/release_gates/contracts/security_layer_inventory.json', 'validation/release_gates'],
  ['client/runtime/config/ownership_drift_policy.json', 'validation/release_gates/contracts/ownership_drift_policy.json', 'validation/release_gates'],
  ['client/runtime/config/command_registry_policy.json', 'tests/tooling/config/command_registry_policy.json', 'tests/tooling'],
  ['client/runtime/config/command_registry.json', 'tests/tooling/config/command_registry.json', 'tests/tooling'],
  ['client/runtime/config/lane_command_registry.json', 'tests/tooling/config/lane_command_registry.json', 'tests/tooling'],
  ['client/runtime/config/agent_routing_rules.json', 'orchestration/config/agent_routing_rules.json', 'orchestration'],
  ['client/runtime/config/workflow_executor_policy.json', 'orchestration/config/workflow_executor_policy.json', 'orchestration'],
  ['client/runtime/config/orchestration_workflow_contract_policy.json', 'orchestration/config/orchestration_workflow_contract_policy.json', 'orchestration'],
  ['client/runtime/config/orchestration_quality_policy.json', 'orchestration/config/orchestration_quality_policy.json', 'orchestration'],
  ['client/runtime/config/orchestration_ownership_policy.json', 'orchestration/config/orchestration_ownership_policy.json', 'orchestration'],
  ['client/runtime/config/orchestration_ts_boundary_policy.json', 'orchestration/config/orchestration_ts_boundary_policy.json', 'orchestration'],
  ['client/runtime/config/orchestration_naming_policy.json', 'orchestration/config/orchestration_naming_policy.json', 'orchestration'],
  ['client/runtime/config/provider_onboarding_manifest.json', 'orchestration/config/provider_onboarding_manifest.json', 'orchestration'],
  ['client/runtime/config/research_plane_policy.json', 'orchestration/config/research_plane_policy.json', 'orchestration'],
  ['client/runtime/config/spawn_policy.json', 'orchestration/config/spawn_policy.json', 'orchestration'],
  ['client/runtime/config/child_organ_runtime_policy.json', 'orchestration/config/child_organ_runtime_policy.json', 'orchestration'],
  ['client/runtime/config/orchestron_policy.json', 'orchestration/config/orchestron_policy.json', 'orchestration'],
  ['client/runtime/config/guard_check_registry.json', 'validation/release_gates/contracts/guard_check_registry.json', 'validation/release_gates'],
  ['client/runtime/config/gateway_ingress_egress_contract.json', 'validation/conformance/contracts/gateway_ingress_egress_contract.json', 'validation/conformance'],
  ['client/runtime/config/interface_payload_budget_contract.json', 'validation/conformance/contracts/interface_payload_budget_contract.json', 'validation/conformance'],
  ['client/runtime/config/conduit_scrambler_posture_contract.json', 'validation/conformance/contracts/conduit_scrambler_posture_contract.json', 'validation/conformance'],
  ['client/runtime/config/cross_domain_nexus_route_inventory.json', 'validation/conformance/contracts/cross_domain_nexus_route_inventory.json', 'validation/conformance'],
  ['client/runtime/config/shell_ui_message_detail_contract.json', 'validation/conformance/contracts/shell_ui_message_detail_contract.json', 'validation/conformance'],
  ['client/runtime/config/shell_truth_leak_policy.json', 'validation/conformance/contracts/shell_truth_leak_policy.json', 'validation/conformance'],
  ['client/runtime/config/shell_backend_state_contract.json', 'validation/conformance/contracts/shell_backend_state_contract.json', 'validation/conformance'],
  ['client/runtime/config/shell_naming_policy.json', 'validation/conformance/contracts/shell_naming_policy.json', 'validation/conformance'],
];

const AUTHORITY_CONFIG_NAME = /(?:policy|rules|manifest|gate|executor|routing|abac|permission|security|workflow|provider|tool|gateway|conduit|research|batch|memory|command|authority)/i;

function readArg(name: string, fallback = ''): string {
  const prefix = `--${name}=`;
  const hit = process.argv.slice(2).find((arg) => String(arg).startsWith(prefix));
  return hit ? String(hit).slice(prefix.length) : fallback;
}

function boolArg(name: string, fallback = false): boolean {
  const raw = readArg(name, fallback ? '1' : '0').trim().toLowerCase();
  return raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on';
}

function abs(rel: string): string {
  return path.join(ROOT, rel);
}

function readText(rel: string): string {
  return fs.readFileSync(abs(rel), 'utf8');
}

function readJson(rel: string): any {
  return JSON.parse(readText(rel));
}

function push(checks: Check[], id: string, ok: boolean, detail: string): void {
  checks.push({ id, ok, detail });
}

function writeJson(filePath: string, value: any): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function runGit(args: string[]): string {
  try {
    return childProcess.execFileSync('git', args, {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    });
  } catch {
    return '';
  }
}

function gitRefExists(ref: string): boolean {
  return runGit(['rev-parse', '--verify', '--quiet', ref]).trim().length > 0;
}

function resolveDiffBaseRef(): string {
  const explicit = readArg('base-ref', String(process.env.SHELL_AUTHORITY_CONFIG_BASE_REF || '')).trim();
  if (explicit && gitRefExists(explicit)) return explicit;
  for (const candidate of ['origin/main', 'main']) {
    if (gitRefExists(candidate)) return candidate;
  }
  return '';
}

function addedClientRuntimeConfigFiles(baseRef: string): string[] {
  const args = baseRef
    ? ['diff', '--name-status', '--diff-filter=A', `${baseRef}...HEAD`, '--', 'client/runtime/config']
    : ['diff', '--name-status', '--diff-filter=A', 'HEAD', '--', 'client/runtime/config'];
  return runGit(args)
    .split(/\r?\n/)
    .map((line: string) => line.trim())
    .filter(Boolean)
    .map((line: string) => line.split(/\s+/).pop() || '')
    .filter((rel: string) => rel.endsWith('.json'));
}

function isAuthorityShapedConfig(rel: string): boolean {
  return AUTHORITY_CONFIG_NAME.test(path.basename(rel));
}

function isCompatibilityMirror(rel: string): boolean {
  try {
    const json = readJson(rel);
    const marked = json.compatibility_mirror === true || json.legacy_mirror === true;
    return marked && typeof json.canonical_path === 'string' && typeof json.canonical_owner === 'string';
  } catch {
    return false;
  }
}

function main(): number {
  const checks: Check[] = [];
  for (const [clientRel, canonicalRel, owner] of MIRROR_CONFIGS) {
    const client = readJson(clientRel);
    const canonical = readJson(canonicalRel);
    push(
      checks,
      `mirror_declares_canonical_path:${clientRel}`,
      client.canonical_path === canonicalRel,
      `${client.canonical_path || 'missing'}`
    );
    push(
      checks,
      `mirror_declares_canonical_owner:${clientRel}`,
      client.canonical_owner === owner,
      `${client.canonical_owner || 'missing'}`
    );
    push(
      checks,
      `mirror_marked_compatibility_only:${clientRel}`,
      client.compatibility_mirror === true,
      `compatibility_mirror=${client.compatibility_mirror === true}`
    );
    push(
      checks,
      `canonical_declares_owner:${canonicalRel}`,
      canonical.canonical_owner === owner,
      `${canonical.canonical_owner || 'missing'}`
    );
  }

  const stateManifest = readJson('core/layer0/ops/config/state_tier_manifest.json');
  const clientAuthorities = (stateManifest.hot_runtime || [])
    .map((entry: any) => String(entry.authority || ''))
    .filter((authority: string) => authority.startsWith('client/'));
  push(
    checks,
    'state_tier_manifest_has_no_client_authority_paths',
    clientAuthorities.length === 0,
    clientAuthorities.join(', ') || 'none'
  );

  const baseRef = resolveDiffBaseRef();
  const addedAuthorityConfigs = addedClientRuntimeConfigFiles(baseRef).filter(isAuthorityShapedConfig);
  const unmarkedAuthorityConfigs = addedAuthorityConfigs.filter((rel: string) => !isCompatibilityMirror(rel));
  push(
    checks,
    'new_client_runtime_authority_configs_are_declared_mirrors',
    unmarkedAuthorityConfigs.length === 0,
    `base=${baseRef || 'HEAD'} added_authority_configs=${addedAuthorityConfigs.length} unmarked=${unmarkedAuthorityConfigs.join(', ') || 'none'}`
  );

  const runtimeEntrypoint = readText('client/runtime/lib/runtime_system_entrypoint.ts');
  for (const forbidden of [
    'MUTATING_ACTIONS',
    'READ_ONLY_ACTIONS',
    'classifyBridgeError',
    'retryHintsForErrorClass',
    'inferBridgeErrorCode',
    'detectMutationLikely',
  ]) {
    push(
      checks,
      `runtime_entrypoint_no_shell_authority_token:${forbidden}`,
      !runtimeEntrypoint.includes(forbidden),
      forbidden
    );
  }
  push(
    checks,
    'runtime_entrypoint_uses_core_authority_context',
    runtimeEntrypoint.includes('entrypoint-context') &&
      runtimeEntrypoint.includes('runtimeEntrypointAuthorityContext'),
    'runtime-systems entrypoint-context'
  );

  const stateTiering = readText('client/runtime/systems/ops/state_tiering_contract.ts');
  push(
    checks,
    'state_tiering_contract_defaults_to_core_manifest',
    stateTiering.includes("policy: 'core/layer0/ops/config/state_tier_manifest.json'"),
    'core/layer0/ops/config/state_tier_manifest.json'
  );

  const egressGateway = readText('client/runtime/lib/egress_gateway.ts');
  push(
    checks,
    'egress_gateway_defaults_to_core_policy',
    egressGateway.includes("const DEFAULT_POLICY_REL = 'core/layer0/ops/config/egress_gateway_policy.json'"),
    'core/layer0/ops/config/egress_gateway_policy.json'
  );

  const collectorRuntime = readText(
    'client/cognition/shared/adaptive/sensory/eyes/collectors/collector_runtime.ts'
  );
  push(
    checks,
    'collector_runtime_uses_core_egress_policy',
    collectorRuntime.includes("'core',") &&
      collectorRuntime.includes("'layer0',") &&
      collectorRuntime.includes("'ops',") &&
      collectorRuntime.includes("'egress_gateway_policy.json'"),
    'core/layer0/ops/config/egress_gateway_policy.json'
  );

  const failures = checks.filter((row) => !row.ok);
  const report = {
    ok: failures.length === 0,
    type: 'shell_authority_config_guard',
    generated_at: new Date().toISOString(),
    summary: {
      check_count: checks.length,
      failure_count: failures.length,
      pass: failures.length === 0,
    },
    checks,
  };

  writeJson(abs(readArg('out-json', 'core/local/artifacts/shell_authority_config_guard_current.json')), report);
  process.stdout.write(`${JSON.stringify(report.summary)}\n`);
  return failures.length > 0 && boolArg('strict', false) ? 1 : 0;
}

process.exit(main());
