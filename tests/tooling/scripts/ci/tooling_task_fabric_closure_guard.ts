import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname } from 'path';

const SRS_ID = 'V12-TOOL-TASK-FABRIC-CLOSURE-001';
const LEGACY_SRS_ID = 'V11-TOOL-006';
const MANIFEST = 'tests/tooling/config/release_proof_pack_manifest.json';
const REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const OUT_JSON = 'core/local/artifacts/tooling_task_fabric_closure_guard_current.json';
const OUT_MARKDOWN = 'local/workspace/reports/TOOLING_TASK_FABRIC_CLOSURE_GUARD_CURRENT.md';
const GATE_ID = 'ops:tooling-task-fabric:closure:guard';

const REQUIRED_ARTIFACTS = [
  'core/local/artifacts/typed_probe_contract_matrix_guard_current.json',
  'core/local/artifacts/transport_convergence_guard_current.json',
  'core/local/artifacts/transport_spawn_audit_current.json',
  'core/local/artifacts/tool_route_decision_current.json',
  'core/local/artifacts/workspace_tooling_release_proof_current.json',
  'core/local/artifacts/workspace_tooling_context_soak_current.json',
  'core/local/artifacts/web_tooling_reliability_current.json',
  'core/local/artifacts/release_policy_gate_current.json',
];

const TYPED_PROBE_KEYS = ['workspace_read', 'workspace_search', 'web_search', 'web_fetch', 'tool_route'];

type Check = { id: string; ok: boolean; detail?: string };

function arg(name: string, fallback: string): string {
  const prefix = `--${name}=`;
  return process.argv.find((item) => item.startsWith(prefix))?.slice(prefix.length) ?? fallback;
}

function flag(name: string, fallback: boolean): boolean {
  const value = arg(name, fallback ? '1' : '0').toLowerCase();
  return value === '1' || value === 'true' || value === 'yes';
}

function readJson(path: string): any {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function readJsonMaybe(path: string): any | null {
  return existsSync(path) ? readJson(path) : null;
}

function readText(path: string): string {
  return readFileSync(path, 'utf8');
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

function registryArtifacts(registry: any, gateId: string): string[] {
  return list(registry?.gates?.[gateId]?.artifact_paths);
}

function registryRunnable(registry: any, gateId: string): boolean {
  const entry = registry?.gates?.[gateId];
  return Boolean(entry && (Array.isArray(entry.command) || typeof entry.script === 'string'));
}

function requiredArtifacts(manifest: any): string[] {
  return list(manifest?.required_artifacts);
}

function releaseGovernanceArtifacts(manifest: any): string[] {
  return list(manifest?.artifact_groups?.release_governance);
}

function adapterAndOrchestrationArtifacts(manifest: any): string[] {
  return list(manifest?.artifact_groups?.adapter_and_orchestration);
}

function workloadAndQualityArtifacts(manifest: any): string[] {
  return list(manifest?.artifact_groups?.workload_and_quality);
}

function artifactPasses(payload: any): boolean {
  return payload?.ok === true || payload?.summary?.pass === true || payload?.summary?.violation_count === 0;
}

function hasZeroSummary(payload: any, key: string): boolean {
  return Number(payload?.summary?.[key] ?? 1) === 0;
}

function sourceHas(path: string, tokens: string[]): Check[] {
  const source = readText(path);
  return tokens.map((token) => check(`source_contract:${path}:${token}`, source.includes(token), token));
}

function writeMarkdown(path: string, checks: Check[], pass: boolean): void {
  ensureParent(path);
  const lines = [
    '# Tooling + Task Fabric Closure Guard',
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
  const adapterAndOrchestration = adapterAndOrchestrationArtifacts(manifest);
  const workloadAndQuality = workloadAndQualityArtifacts(manifest);
  const typedProbe = readJsonMaybe(REQUIRED_ARTIFACTS[0]);
  const transport = readJsonMaybe(REQUIRED_ARTIFACTS[1]);
  const spawnAudit = readJsonMaybe(REQUIRED_ARTIFACTS[2]);
  const toolRoute = readJsonMaybe(REQUIRED_ARTIFACTS[3]);
  const workspaceRelease = readJsonMaybe(REQUIRED_ARTIFACTS[4]);
  const workspaceSoak = readJsonMaybe(REQUIRED_ARTIFACTS[5]);
  const webReliability = readJsonMaybe(REQUIRED_ARTIFACTS[6]);
  const releasePolicy = readJsonMaybe(REQUIRED_ARTIFACTS[7]);
  const checks: Check[] = [
    check('closure_guard_registered_as_release_governance_artifact', releaseGovernance.includes(OUT_JSON), OUT_JSON),
    check('closure_guard_required_in_proof_pack', required.includes(OUT_JSON), OUT_JSON),
    check('closure_guard_markdown_registry_exported', registryArtifacts(registry, GATE_ID).includes(OUT_MARKDOWN), OUT_MARKDOWN),
    check('closure_guard_registry_entry_runnable', registryRunnable(registry, GATE_ID)),
  ];
  for (const path of REQUIRED_ARTIFACTS) {
    checks.push(check(`tooling_artifact_required:${path}`, required.includes(path), path));
    checks.push(
      check(
        `tooling_artifact_grouped:${path}`,
        adapterAndOrchestration.includes(path) || workloadAndQuality.includes(path) || releaseGovernance.includes(path),
        path,
      ),
    );
    checks.push(check(`tooling_artifact_exists:${path}`, existsSync(path), path));
  }
  checks.push(check('typed_probe_matrix_passes', artifactPasses(typedProbe)));
  for (const key of TYPED_PROBE_KEYS) {
    const checksJson = JSON.stringify(typedProbe?.checks ?? []);
    checks.push(check(`typed_probe_key_covered:${key}`, checksJson.includes(key), key));
  }
  checks.push(check('transport_convergence_passes', artifactPasses(transport)));
  checks.push(check('transport_convergence_zero_violations', Number(transport?.summary?.violation_count ?? 1) === 0));
  checks.push(check('transport_spawn_audit_passes', artifactPasses(spawnAudit)));
  checks.push(check('transport_spawn_no_runtime_hot_path', hasZeroSummary(spawnAudit, 'runtime_hot_path')));
  checks.push(check('transport_spawn_no_wrapper_candidates', hasZeroSummary(spawnAudit, 'wrapper_candidates')));
  checks.push(check('tool_route_misdirection_guard_passes', artifactPasses(toolRoute)));
  checks.push(check('tool_route_misdirection_zero', Number(toolRoute?.summary?.misdirection_count ?? 1) === 0));
  checks.push(check('tool_route_missing_evidence_zero', Number(toolRoute?.summary?.missing_evidence_cases ?? 1) === 0));
  checks.push(check('workspace_tooling_release_proof_passes', artifactPasses(workspaceRelease)));
  checks.push(check('workspace_tooling_soak_passes', artifactPasses(workspaceSoak)));
  checks.push(check('web_tooling_reliability_passes', artifactPasses(webReliability)));
  checks.push(check('release_policy_gate_passes', artifactPasses(releasePolicy)));
  checks.push(
    ...sourceHas('core/layer2/tools/task_fabric/src/concurrency.rs', [
      'pub proof_refs: Vec<String>',
      'validate_proof_refs',
      'proof_refs_required',
    ]),
    ...sourceHas('core/layer2/tools/task_fabric/src/lib.rs', [
      'validate_proof_refs(&envelope.proof_refs)',
      'proof_refs: proof_refs.clone()',
      'task_fabric_receipt_v1',
    ]),
    ...sourceHas('adapters/runtime/run_infring_ops.ts', [
      'process_fallback_forbidden_in_production',
      "envOverrides.INFRING_OPS_ALLOW_PROCESS_FALLBACK = '0'",
      "envOverrides.INFRING_SDK_ALLOW_PROCESS_TRANSPORT = '0'",
    ]),
  );
  const pass = checks.every((row) => row.ok);
  const payload = {
    ok: pass,
    type: 'tooling_task_fabric_closure_guard',
    srs_id: SRS_ID,
    legacy_srs_ids: [LEGACY_SRS_ID],
    generated_at: new Date().toISOString(),
    inputs: { manifest_path: manifestPath, registry_path: registryPath },
    summary: {
      pass,
      check_count: checks.length,
      required_artifact_count: REQUIRED_ARTIFACTS.length,
      typed_probe_key_count: TYPED_PROBE_KEYS.length,
    },
    checks,
    artifact_paths: [outJson, outMarkdown],
  };
  ensureParent(outJson);
  writeFileSync(outJson, `${JSON.stringify(payload, null, 2)}\n`);
  writeMarkdown(outMarkdown, checks, pass);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && !pass) process.exit(1);
}

main();
