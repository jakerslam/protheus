import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname } from 'path';

const SRS_ID = 'V12-CLIENT-DEAUTHORITY-CLOSURE-001';
const LEGACY_SRS_ID = 'V11-CLIENT-009';
const MANIFEST = 'tests/tooling/config/release_proof_pack_manifest.json';
const REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const POLICY = 'client/runtime/config/shell_truth_leak_policy.json';
const FIXTURE = 'tests/tooling/fixtures/workflow_failure_recovery_matrix.json';
const RECOVERY_GUARD = 'tests/tooling/scripts/ci/workflow_failure_recovery_guard.ts';
const OUT_JSON = 'core/local/artifacts/client_deauthority_closure_guard_current.json';
const OUT_MARKDOWN = 'local/workspace/reports/CLIENT_DEAUTHORITY_CLOSURE_GUARD_CURRENT.md';
const GATE_ID = 'ops:client-deauthority:closure:guard';

const REQUIRED_GATES = [
  'ops:shell:truth-leak:guard',
  'ops:workflow-failure-recovery:guard',
];

const REQUIRED_ARTIFACTS = [
  'core/local/artifacts/shell_truth_leak_guard_current.json',
  'core/local/artifacts/workflow_failure_recovery_current.json',
];

const REQUIRED_POLICY_IDS = [
  'workflow_unexpected_state_retry_loop',
  'workflow_gate_unexpected_state_retry_boilerplate',
  'workflow_gate_unexpected_state_short_boilerplate',
  'workflow_final_reply_not_rendered_retry_boilerplate',
  'workflow_final_reply_not_rendered_short_boilerplate',
  'workflow_retry_loop_next_actions_boilerplate',
  'workflow_retry_loop_next_actions_unordered_boilerplate',
  'workflow_retry_loop_rerun_chain_template',
  'automatic_tool_trigger_admission',
  'automatic_tool_trigger_backend_automation_admission',
  'automatic_tool_trigger_still_active_admission',
  'backend_automation_overrides_semantic_intent_admission',
  'file_list_raw_lease_denied_echo',
  'file_list_raw_lease_denied_echo_unquoted',
  'file_list_policy_gate_trace_blocked_template',
  'file_list_policy_gate_trace_blocked_template_generic',
  'workspace_file_policy_gate_template_loop',
  'file_tooling_validation_drift_template',
  'workflow_route_info_task_classification_boilerplate',
  'workflow_route_task_classification_path_boilerplate',
];

const REQUIRED_FIXTURE_CASES = [
  'policy_gate_outage_template_loop_blocked',
  'ingress_domain_boundary_template_loop_blocked',
  'paraphrased_retry_macro_loop_blocked',
  'exact_duplicate_response_loop_blocked',
  'alternating_retry_templates_loop_blocked',
];

const REQUIRED_RECOVERY_TOKENS = [
  'policy_gate_outage_template',
  'runtime_capability_surface_template',
  'workflow_retry_macro_template',
  'final_reply_retry_template',
  'exact_repeat',
  'alternating_repeat',
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

function requiredArtifacts(manifest: any): string[] {
  return list(manifest?.required_artifacts);
}

function workloadArtifacts(manifest: any): string[] {
  return list(manifest?.artifact_groups?.workload_and_quality);
}

function registryArtifacts(registry: any, gateId: string): string[] {
  return list(registry?.gates?.[gateId]?.artifact_paths);
}

function registryRunnable(registry: any, gateId: string): boolean {
  const entry = registry?.gates?.[gateId];
  return Boolean(entry && (Array.isArray(entry.command) || typeof entry.script === 'string'));
}

function forbiddenPatternIds(policy: any): string[] {
  return Array.isArray(policy?.forbidden_patterns)
    ? policy.forbidden_patterns
        .map((row: any) => (typeof row?.id === 'string' ? row.id : ''))
        .filter(Boolean)
    : [];
}

function policyErrorEnforcedIds(policy: any): string[] {
  const nativeErrors = Array.isArray(policy?.forbidden_patterns)
    ? policy.forbidden_patterns
        .filter((row: any) => row?.severity === 'error')
        .map((row: any) => (typeof row?.id === 'string' ? row.id : ''))
        .filter(Boolean)
    : [];
  return [...new Set([...nativeErrors, ...list(policy?.error_on_warning_pattern_ids)])];
}

function fixtureCaseIds(fixture: any): string[] {
  return Array.isArray(fixture?.cases)
    ? fixture.cases
        .map((row: any) => (typeof row?.id === 'string' ? row.id : ''))
        .filter(Boolean)
    : [];
}

function writeMarkdown(path: string, checks: Check[], pass: boolean): void {
  ensureParent(path);
  const lines = [
    '# Client De-Authority Closure Guard',
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
  const fixturePath = arg('fixture', FIXTURE);
  const recoveryGuardPath = arg('recovery-guard', RECOVERY_GUARD);
  const outJson = arg('out-json', OUT_JSON);
  const outMarkdown = arg('out-markdown', OUT_MARKDOWN);
  const strict = flag('strict', true);

  const manifest = readJson(manifestPath);
  const registry = readJson(registryPath);
  const policy = readJson(policyPath);
  const fixture = readJson(fixturePath);
  const recoveryGuardSource = readText(recoveryGuardPath);
  const required = requiredArtifacts(manifest);
  const workload = workloadArtifacts(manifest);
  const patternIds = forbiddenPatternIds(policy);
  const requiredPatternIds = list(policy?.required_pattern_ids);
  const errorEnforcedIds = policyErrorEnforcedIds(policy);
  const cases = fixtureCaseIds(fixture);

  const checks: Check[] = [
    check('closure_guard_required_in_proof_pack', required.includes(OUT_JSON), OUT_JSON),
    check('closure_guard_grouped_as_workload_quality', workload.includes(OUT_JSON), OUT_JSON),
    check('closure_guard_markdown_registry_exported', registryArtifacts(registry, GATE_ID).includes(OUT_MARKDOWN), OUT_MARKDOWN),
    check('closure_guard_registry_entry_runnable', registryRunnable(registry, GATE_ID)),
  ];

  for (const gateId of REQUIRED_GATES) {
    checks.push(check(`required_gate_registered:${gateId}`, registryRunnable(registry, gateId), gateId));
  }

  for (const artifact of REQUIRED_ARTIFACTS) {
    checks.push(check(`required_evidence_artifact_in_manifest:${artifact}`, required.includes(artifact), artifact));
  }

  for (const id of REQUIRED_POLICY_IDS) {
    checks.push(check(`policy_pattern_declared:${id}`, patternIds.includes(id), id));
    checks.push(check(`policy_pattern_required:${id}`, requiredPatternIds.includes(id), id));
    checks.push(check(`policy_pattern_error_enforced:${id}`, errorEnforcedIds.includes(id), id));
  }

  for (const id of REQUIRED_FIXTURE_CASES) {
    checks.push(check(`workflow_recovery_fixture_case:${id}`, cases.includes(id), id));
  }

  for (const token of REQUIRED_RECOVERY_TOKENS) {
    checks.push(check(`workflow_recovery_detection_token:${token}`, recoveryGuardSource.includes(token), token));
  }

  checks.push(
    check(
      'workflow_recovery_fixture_requires_degraded_on_finalization_failure',
      fixture?.recovery_policy?.require_degraded_on_finalization_failure === true,
    ),
  );
  checks.push(
    check(
      'workflow_recovery_fixture_bounds_retries',
      Number(fixture?.recovery_policy?.max_retry) <= 2,
      `max_retry=${fixture?.recovery_policy?.max_retry}`,
    ),
  );

  const pass = checks.every((row) => row.ok);
  const payload = {
    ok: pass,
    type: 'client_deauthority_closure_guard',
    srs_id: SRS_ID,
    legacy_srs_ids: [LEGACY_SRS_ID],
    generated_at: new Date().toISOString(),
    inputs: {
      manifest_path: manifestPath,
      registry_path: registryPath,
      policy_path: policyPath,
      fixture_path: fixturePath,
      recovery_guard_path: recoveryGuardPath,
    },
    summary: {
      pass,
      check_count: checks.length,
      required_policy_pattern_count: REQUIRED_POLICY_IDS.length,
      required_fixture_case_count: REQUIRED_FIXTURE_CASES.length,
      required_recovery_token_count: REQUIRED_RECOVERY_TOKENS.length,
    },
    checks,
    artifact_paths: [outJson, outMarkdown],
  };

  ensureParent(outJson);
  writeFileSync(outJson, `${JSON.stringify(payload, null, 2)}\n`);
  writeMarkdown(outMarkdown, checks, pass);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && !pass) {
    process.exitCode = 1;
  }
}

main();
