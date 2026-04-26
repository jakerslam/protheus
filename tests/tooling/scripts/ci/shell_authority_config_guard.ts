#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');

type Check = {
  id: string;
  ok: boolean;
  detail: string;
};

const MIRROR_CONFIGS = [
  ['client/runtime/config/state_tier_manifest.json', 'core/layer0/ops/config/state_tier_manifest.json', 'core/layer0/ops'],
  ['client/runtime/config/egress_gateway_policy.json', 'core/layer0/ops/config/egress_gateway_policy.json', 'core/layer0/ops'],
  ['client/runtime/config/abac_policy_plane.json', 'core/layer1/security/config/abac_policy_plane.json', 'core/layer1/security'],
  ['client/runtime/config/agent_routing_rules.json', 'surface/orchestration/config/agent_routing_rules.json', 'surface/orchestration'],
  ['client/runtime/config/workflow_executor_policy.json', 'surface/orchestration/config/workflow_executor_policy.json', 'surface/orchestration'],
  ['client/runtime/config/provider_onboarding_manifest.json', 'surface/orchestration/config/provider_onboarding_manifest.json', 'surface/orchestration'],
];

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
