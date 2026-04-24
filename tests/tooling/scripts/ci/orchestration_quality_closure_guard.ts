import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname } from 'path';

const SRS_ID = 'V12-ORCH-QUALITY-CLOSURE-001';
const LEGACY_SRS_ID = 'V11-ORCH-007';
const MANIFEST = 'tests/tooling/config/release_proof_pack_manifest.json';
const REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const POLICY = 'client/runtime/config/orchestration_quality_policy.json';
const OUT_JSON = 'core/local/artifacts/orchestration_quality_closure_guard_current.json';
const OUT_MARKDOWN = 'local/workspace/reports/ORCHESTRATION_QUALITY_CLOSURE_GUARD_CURRENT.md';
const GATE_ID = 'ops:orchestration:quality-closure:guard';

const REQUIRED_ARTIFACTS = [
  'core/local/artifacts/orchestration_planner_quality_guard_current.json',
  'core/local/artifacts/orchestration_runtime_quality_guard_current.json',
  'core/local/artifacts/orchestration_gateway_fallback_guard_current.json',
  'core/local/artifacts/orchestration_workflow_contract_guard_current.json',
];
const REQUIRED_PLANNER_FIELDS = [
  'request_count',
  'average_candidate_count',
  'clarification_first_rate',
  'degraded_rate',
  'selected_plan_requires_clarification_rate',
  'selected_plan_degraded_rate',
  'heuristic_probe_rate',
  'zero_executable_candidate_rate',
  'all_candidates_require_clarification_rate',
  'all_candidates_degraded_rate',
];
const REQUIRED_RUNTIME_FIELDS = [
  'sample_size_non_legacy',
  'fallback_rate_non_legacy',
  'heuristic_probe_rate_non_legacy',
  'clarification_rate_non_legacy',
  'zero_executable_rate_non_legacy',
  'all_candidates_degraded_rate_non_legacy',
  'average_candidate_count',
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

function readJson(path: string): any {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function readJsonMaybe(path: string): any | null {
  return existsSync(path) ? readJson(path) : null;
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

function requiredArtifacts(manifest: any): string[] {
  return list(manifest?.required_artifacts);
}

function adapterAndOrchestrationArtifacts(manifest: any): string[] {
  return list(manifest?.artifact_groups?.adapter_and_orchestration);
}

function releaseGovernanceArtifacts(manifest: any): string[] {
  return list(manifest?.artifact_groups?.release_governance);
}

function registryArtifacts(registry: any, gateId: string): string[] {
  return list(registry?.gates?.[gateId]?.artifact_paths);
}

function registryRunnable(registry: any, gateId: string): boolean {
  const entry = registry?.gates?.[gateId];
  return Boolean(entry && (Array.isArray(entry.command) || typeof entry.script === 'string'));
}

function number(value: unknown): number | null {
  return Number.isFinite(Number(value)) ? Number(value) : null;
}

function fieldsPresent(metrics: Record<string, unknown> | undefined, fields: string[]): boolean {
  if (!metrics || typeof metrics !== 'object') return false;
  return fields.every((field) => number(metrics[field]) != null);
}

function maxZero(metrics: Record<string, unknown> | undefined, fields: string[]): boolean {
  if (!metrics || typeof metrics !== 'object') return false;
  return fields.every((field) => Number(metrics[field] ?? 1) === 0);
}

function policyHasStrictMetricContract(policy: any): boolean {
  const planner = policy?.planner_quality;
  const runtime = policy?.runtime_quality;
  return (
    Array.isArray(planner?.required_metric_fields) &&
    REQUIRED_PLANNER_FIELDS.every((field) => planner.required_metric_fields.includes(field)) &&
    Number(planner?.max_missing_metric_fields) === 0 &&
    Number(planner?.max_heuristic_probe_rate) <= 0.1 &&
    Number(planner?.max_zero_executable_candidate_rate) <= 0.05 &&
    Array.isArray(runtime?.required_metric_fields) &&
    REQUIRED_RUNTIME_FIELDS.every((field) => runtime.required_metric_fields.includes(field)) &&
    Number(runtime?.max_missing_metric_fields) === 0 &&
    Number(runtime?.max_non_legacy_heuristic_probe_rate) <= 0.1 &&
    Number(runtime?.max_non_legacy_zero_executable_rate) <= 0.05
  );
}

function writeMarkdown(path: string, checks: Check[], pass: boolean): void {
  ensureParent(path);
  const lines = [
    '# Orchestration Quality Closure Guard',
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
  const policyPath = arg('policy', POLICY);
  const outJson = arg('out-json', OUT_JSON);
  const outMarkdown = arg('out-markdown', OUT_MARKDOWN);
  const strict = flag('strict', true);
  const manifest = readJson(manifestPath);
  const registry = readJson(registryPath);
  const policy = readJson(policyPath);
  const required = requiredArtifacts(manifest);
  const orchestrationGroup = adapterAndOrchestrationArtifacts(manifest);
  const releaseGovernance = releaseGovernanceArtifacts(manifest);
  const planner = readJsonMaybe(REQUIRED_ARTIFACTS[0]);
  const runtime = readJsonMaybe(REQUIRED_ARTIFACTS[1]);
  const gateway = readJsonMaybe(REQUIRED_ARTIFACTS[2]);
  const workflow = readJsonMaybe(REQUIRED_ARTIFACTS[3]);
  const checks: Check[] = [
    check('closure_guard_registered_as_release_governance_artifact', releaseGovernance.includes(OUT_JSON), OUT_JSON),
    check('closure_guard_required_in_proof_pack', required.includes(OUT_JSON), OUT_JSON),
    check('closure_guard_markdown_registry_exported', registryArtifacts(registry, GATE_ID).includes(OUT_MARKDOWN), OUT_MARKDOWN),
    check('closure_guard_registry_entry_runnable', registryRunnable(registry, GATE_ID)),
    check('orchestration_policy_has_strict_required_metric_contract', policyHasStrictMetricContract(policy)),
  ];
  for (const path of REQUIRED_ARTIFACTS) {
    checks.push(check(`orchestration_artifact_required:${path}`, required.includes(path), path));
    checks.push(check(`orchestration_artifact_grouped:${path}`, orchestrationGroup.includes(path), path));
    checks.push(check(`orchestration_artifact_exists:${path}`, existsSync(path), path));
  }
  checks.push(check('planner_guard_passes', planner?.ok === true && planner?.summary?.pass === true));
  checks.push(check('planner_guard_metrics_complete', fieldsPresent(planner?.metrics, REQUIRED_PLANNER_FIELDS)));
  checks.push(
    check(
      'planner_guard_zero_regression_metrics',
      maxZero(planner?.metrics, [
        'heuristic_probe_rate',
        'zero_executable_candidate_rate',
        'all_candidates_degraded_rate',
      ]),
    ),
  );
  checks.push(check('runtime_guard_passes', runtime?.ok === true && runtime?.summary?.pass === true));
  checks.push(check('runtime_guard_metrics_complete', fieldsPresent(runtime?.metrics, REQUIRED_RUNTIME_FIELDS)));
  checks.push(
    check(
      'runtime_guard_zero_regression_metrics',
      maxZero(runtime?.metrics, [
        'fallback_rate_non_legacy',
        'heuristic_probe_rate_non_legacy',
        'zero_executable_rate_non_legacy',
        'all_candidates_degraded_rate_non_legacy',
      ]),
    ),
  );
  checks.push(check('gateway_fallback_guard_passes', gateway?.ok === true && gateway?.summary?.pass === true));
  checks.push(check('workflow_contract_guard_passes', workflow?.ok === true && workflow?.summary?.pass === true));
  checks.push(
    check(
      'gateway_surface_metrics_complete',
      Number(gateway?.summary?.surface_rows_with_complete_metrics) >= 3 &&
        Number(gateway?.summary?.missing_surface_field_count) === 0,
    ),
  );
  const pass = checks.every((row) => row.ok);
  const payload = {
    ok: pass,
    type: 'orchestration_quality_closure_guard',
    srs_id: SRS_ID,
    legacy_srs_ids: [LEGACY_SRS_ID],
    generated_at: new Date().toISOString(),
    inputs: { manifest_path: manifestPath, registry_path: registryPath, policy_path: policyPath },
    summary: {
      pass,
      check_count: checks.length,
      required_artifact_count: REQUIRED_ARTIFACTS.length,
      planner_metric_count: REQUIRED_PLANNER_FIELDS.length,
      runtime_metric_count: REQUIRED_RUNTIME_FIELDS.length,
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
