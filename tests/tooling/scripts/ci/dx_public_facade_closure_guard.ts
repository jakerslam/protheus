import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname } from 'path';

const SRS_ID = 'V12-DX-PUBLIC-FACADE-CLOSURE-001';
const LEGACY_SRS_ID = 'V11-DX-002';
const MANIFEST = 'tests/tooling/config/release_proof_pack_manifest.json';
const REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const OUT_JSON = 'core/local/artifacts/dx_public_facade_closure_guard_current.json';
const OUT_MARKDOWN = 'local/workspace/reports/DX_PUBLIC_FACADE_CLOSURE_GUARD_CURRENT.md';
const GATE_ID = 'ops:dx-public-facade:closure:guard';

const CORE_INDEX = 'packages/infring-core/index.ts';
const EDGE_INDEX = 'packages/infring-edge/index.ts';
const CORE_PACKAGE = 'packages/infring-core/package.json';
const EDGE_PACKAGE = 'packages/infring-edge/package.json';
const CORE_README = 'packages/infring-core/README.md';
const EDGE_README = 'packages/infring-edge/README.md';
const FACADE_TEST = 'tests/client-memory-tools/infring_package_facades.test.ts';
const BRIDGE_TEST = 'tests/client-memory-tools/infring_kernel_bridge.test.ts';
const RELEASE_CONTRACT_GATE = 'tests/tooling/scripts/ci/release_contract_gate.ts';
const PRODUCTION_CLOSURE_GATE = 'tests/tooling/scripts/ci/production_readiness_closure_gate.ts';
const PRODUCTION_CLOSURE_POLICY = 'client/runtime/config/production_readiness_closure_policy.json';

const REQUIRED_GATES = [
  'ops:release-contract:gate',
  'ops:production-closure:gate',
];

const REQUIRED_ARTIFACTS = [
  'core/local/artifacts/release_contract_gate_current.json',
  'core/local/artifacts/production_readiness_closure_gate_current.json',
];

const CORE_INDEX_TOKENS = [
  'infring-core-live',
  'invokeTsModuleSync',
  'sanitizeBridgeArg',
  'isPathInsideRoot',
  'bridge_script_outside_root',
  'bridge_export_invalid',
  'spineStatus',
  'reflexStatus',
  'gateStatus',
  'runtime_contract',
  'infring-ops spine status',
  'infring-ops security-plane status',
];

const EDGE_INDEX_TOKENS = [
  'infring_edge_compat_notice',
  'compatibilityStub',
  'edge_runtime_start_removed',
  'edge_swarm_bridge_removed',
  'edge_wrapper_distribution_runtime_removed',
  'MOBILE_ADAPTER',
  'WRAPPER_POLICY',
  'edgeRuntime',
  'edgeLifecycle',
  'edgeWrapper',
  'edgeStatusBundle',
  'deprecated: true',
];

const FACADE_TEST_TOKENS = [
  'core.spineStatus()',
  'core.reflexStatus()',
  'core.gateStatus()',
  "edge.edgeRuntime('status')",
  "edge.edgeLifecycle('status')",
  "edge.edgeWrapper('status'",
  "edge.edgeSwarm('status')",
  'infring_package_facades_test',
];

const BRIDGE_TEST_TOKENS = [
  'invokeKernel',
  'invokeKernelPayload',
  'memory-policy-kernel',
  'infring_kernel_bridge_test',
];

type Check = { id: string; ok: boolean; detail?: string };

function arg(name: string, fallback: string): string {
  const prefix = `--${name}=`;
  return process.argv.find((item) => item.startsWith(prefix))?.slice(prefix.length) ?? fallback;
}

function flag(name: string, fallback: boolean): boolean {
  const value = arg(name, fallback ? '1' : '0').toLowerCase();
  return value === '1' || value === 'true' || value === 'yes';
}

function readText(path: string): string {
  return readFileSync(path, 'utf8');
}

function readJson(path: string): any {
  return JSON.parse(readText(path));
}

function list(value: any): string[] {
  return Array.isArray(value) ? value.filter((item) => typeof item === 'string') : [];
}

function check(id: string, ok: boolean, detail?: string): Check {
  return detail ? { id, ok, detail } : { id, ok };
}

function ensureParent(path: string): void {
  mkdirSync(dirname(path), { recursive: true });
}

function registryGate(registry: any, gateId: string): any {
  return registry?.gates?.[gateId];
}

function registryRunnable(registry: any, gateId: string): boolean {
  const entry = registryGate(registry, gateId);
  return Boolean(entry && (Array.isArray(entry.command) || typeof entry.script === 'string'));
}

function registryArtifacts(registry: any, gateId: string): string[] {
  return list(registryGate(registry, gateId)?.artifact_paths);
}

function registryCommandText(registry: any, gateId: string): string {
  const entry = registryGate(registry, gateId);
  if (Array.isArray(entry?.command)) return entry.command.join(' ');
  if (typeof entry?.script === 'string') return entry.script;
  return '';
}

function packageScriptText(pkg: any): string {
  return Object.values(pkg?.scripts || {})
    .filter((value) => typeof value === 'string')
    .join('\n');
}

function hasAll(source: string, tokens: string[]): string[] {
  return tokens.filter((token) => !source.includes(token));
}

function requiredArtifacts(manifest: any): string[] {
  return list(manifest?.required_artifacts);
}

function releaseGovernanceArtifacts(manifest: any): string[] {
  return list(manifest?.artifact_groups?.release_governance);
}

function workloadArtifacts(manifest: any): string[] {
  return list(manifest?.artifact_groups?.workload_and_quality);
}

function writeMarkdown(path: string, checks: Check[], pass: boolean): void {
  ensureParent(path);
  const lines = [
    '# DX Public Facade Closure Guard',
    '',
    `- pass: ${pass}`,
    `- srs_id: ${SRS_ID}`,
    `- legacy_srs_id: ${LEGACY_SRS_ID}`,
    '',
    '| Check | Status | Detail |',
    '| --- | --- | --- |',
    ...checks.map((row) => `| ${row.id} | ${row.ok ? 'pass' : 'fail'} | ${row.detail ?? ''} |`),
    '',
  ];
  writeFileSync(path, lines.join('\n'));
}

function main(): void {
  const manifestPath = arg('manifest', MANIFEST);
  const registryPath = arg('registry', REGISTRY);
  const outJson = arg('out-json', OUT_JSON);
  const outMarkdown = arg('out-markdown', OUT_MARKDOWN);
  const strict = flag('strict', true);
  const manifest = readJson(manifestPath);
  const registry = readJson(registryPath);
  const required = requiredArtifacts(manifest);
  const releaseGovernance = releaseGovernanceArtifacts(manifest);
  const workload = workloadArtifacts(manifest);
  const coreIndex = readText(CORE_INDEX);
  const edgeIndex = readText(EDGE_INDEX);
  const corePackage = readJson(CORE_PACKAGE);
  const edgePackage = readJson(EDGE_PACKAGE);
  const coreReadme = readText(CORE_README);
  const edgeReadme = readText(EDGE_README);
  const facadeTest = readText(FACADE_TEST);
  const bridgeTest = readText(BRIDGE_TEST);
  const releaseContractGate = readText(RELEASE_CONTRACT_GATE);
  const productionClosureGate = readText(PRODUCTION_CLOSURE_GATE);
  const productionClosurePolicy = readText(PRODUCTION_CLOSURE_POLICY);
  const checks: Check[] = [
    check('closure_guard_required_in_proof_pack', required.includes(OUT_JSON), OUT_JSON),
    check('closure_guard_grouped_as_workload_quality', workload.includes(OUT_JSON), OUT_JSON),
    check('closure_guard_markdown_registry_exported', registryArtifacts(registry, GATE_ID).includes(outMarkdown), outMarkdown),
    check('closure_guard_registry_entry_runnable', registryRunnable(registry, GATE_ID)),
  ];

  for (const gateId of REQUIRED_GATES) {
    checks.push(check(`required_release_gate_registered:${gateId}`, registryRunnable(registry, gateId), gateId));
  }
  for (const artifact of REQUIRED_ARTIFACTS) {
    checks.push(check(`required_release_artifact_in_manifest:${artifact}`, required.includes(artifact), artifact));
    checks.push(
      check(
        `required_release_artifact_grouped:${artifact}`,
        releaseGovernance.includes(artifact),
        artifact,
      ),
    );
  }

  checks.push(check('core_package_name_contract', corePackage?.name === '@infring/core', corePackage?.name));
  checks.push(check('edge_package_name_contract', edgePackage?.name === '@infring/edge', edgePackage?.name));
  checks.push(
    check(
      'core_package_scripts_use_ts_entrypoint',
      packageScriptText(corePackage).includes('client/runtime/lib/ts_entrypoint.ts'),
    ),
  );
  checks.push(
    check(
      'edge_package_scripts_use_ts_entrypoint',
      packageScriptText(edgePackage).includes('client/runtime/lib/ts_entrypoint.ts'),
    ),
  );

  const missingCoreTokens = hasAll(coreIndex, CORE_INDEX_TOKENS);
  checks.push(check('core_facade_live_contract_tokens', missingCoreTokens.length === 0, missingCoreTokens.join(',')));
  const missingEdgeTokens = hasAll(edgeIndex, EDGE_INDEX_TOKENS);
  checks.push(check('edge_facade_supported_subset_tokens', missingEdgeTokens.length === 0, missingEdgeTokens.join(',')));
  const missingFacadeTestTokens = hasAll(facadeTest, FACADE_TEST_TOKENS);
  checks.push(check('facade_regression_test_covers_live_surfaces', missingFacadeTestTokens.length === 0, missingFacadeTestTokens.join(',')));
  const missingBridgeTestTokens = hasAll(bridgeTest, BRIDGE_TEST_TOKENS);
  checks.push(check('kernel_bridge_regression_test_covers_live_bridge', missingBridgeTestTokens.length === 0, missingBridgeTestTokens.join(',')));

  checks.push(
    check(
      'core_readme_declares_live_runtime_contract',
      coreReadme.includes('@infring/core') && coreReadme.includes('spine') && coreReadme.includes('security-plane'),
    ),
  );
  checks.push(
    check(
      'edge_readme_declares_supported_mobile_subset',
      edgeReadme.includes('@infring/edge')
        && edgeReadme.toLowerCase().includes('mobile')
        && edgeReadme.toLowerCase().includes('deprecated'),
    ),
  );

  checks.push(
    check(
      'release_contract_gate_tracks_production_closure',
      releaseContractGate.includes('ops:production-closure:gate')
        && releaseContractGate.includes('core/local/artifacts/production_readiness_closure_gate_current.json'),
    ),
  );
  checks.push(
    check(
      'production_closure_gate_tracks_release_contract',
      productionClosureGate.includes('core/local/artifacts/release_contract_gate_current.json')
        || productionClosureGate.includes('ops:release-contract:gate')
        || (
          productionClosurePolicy.includes('core/local/artifacts/release_contract_gate_current.json')
          || productionClosurePolicy.includes('ops:release-contract:gate')
        ),
    ),
  );
  checks.push(
    check(
      'release_gate_registry_commands_are_explicit',
      REQUIRED_GATES.every((gateId) => registryCommandText(registry, gateId).length > 0),
    ),
  );

  const pass = checks.every((row) => row.ok);
  const payload = {
    ok: pass,
    type: 'dx_public_facade_closure_guard',
    srs_id: SRS_ID,
    legacy_srs_ids: [LEGACY_SRS_ID],
    generated_at: new Date().toISOString(),
    inputs: { manifest_path: manifestPath, registry_path: registryPath },
    summary: {
      pass,
      check_count: checks.length,
      core_token_count: CORE_INDEX_TOKENS.length,
      edge_token_count: EDGE_INDEX_TOKENS.length,
      required_gate_count: REQUIRED_GATES.length,
      required_artifact_count: REQUIRED_ARTIFACTS.length,
    },
    checks,
    artifact_paths: [outJson, outMarkdown],
  };
  ensureParent(outJson);
  writeFileSync(outJson, `${JSON.stringify(payload, null, 2)}\n`);
  writeMarkdown(outMarkdown, checks, pass);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && !pass) process.exitCode = 1;
}

main();
