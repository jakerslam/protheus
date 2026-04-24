#!/usr/bin/env tsx

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname } from 'path';

const SRS_ID = 'V12-GOV-INCIDENT-CLOSURE-001';
const LEGACY_SRS_ID = 'V11-GOV-INC-001';
const GATE_ID = 'ops:incident-governance:gate';
const CLOSURE_GATE_ID = 'ops:incident-governance:closure:guard';
const REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const PROFILES = 'tests/tooling/config/verify_profiles.json';
const MANIFEST = 'tests/tooling/config/release_proof_pack_manifest.json';
const PACKAGE_JSON = 'package.json';
const CI_WORKFLOW = '.github/workflows/ci.yml';
const POLICY = 'client/runtime/config/incident_operations_governance_policy.json';
const OWNER_ROSTER = 'client/runtime/config/incident_owner_roster.json';
const WAIVERS = 'client/runtime/config/incident_operations_governance_waivers.json';
const POLICY_DOC = 'docs/workspace/policy/incident_operations_governance_policy.md';
const PROCESS_DOC = 'docs/workspace/process/incident_response_workflow.md';
const GATE_SCRIPT = 'tests/tooling/scripts/ci/incident_operations_governance_gate.ts';
const GATE_TEST = 'tests/client-memory-tools/incident_operations_governance_gate.test.ts';
const OUT_JSON = 'core/local/artifacts/incident_governance_closure_guard_current.json';
const OUT_MARKDOWN = 'local/workspace/reports/INCIDENT_GOVERNANCE_CLOSURE_GUARD_CURRENT.md';
const INCIDENT_JSON = 'core/local/artifacts/incident_operations_governance_gate_current.json';
const INCIDENT_MARKDOWN = 'local/workspace/reports/INCIDENT_OPERATIONS_GOVERNANCE_GATE_CURRENT.md';

const POLICY_TOKENS = [
  'severity',
  'ownership',
  'escalation',
  'rollback',
  'post_incident_artifacts',
  'waivers_path',
  'incident_owner_roster.json',
];
const PROCESS_TOKENS = [
  'severity',
  'Escalation',
  'Rollback',
  'Closure',
  'Waivers',
];
const GATE_TOKENS = [
  'incident_operations_governance_gate',
  'incident_owner_roster',
  'incident_operations_governance_waivers',
  'missing_fields',
  'duplicate_waiver_id',
  'placeholder_or_invalid_owners',
];
const TEST_TOKENS = [
  'incident_operations_governance_gate.ts',
  'incident-governance-default',
  'expectOk',
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

function packageScript(pkg: any, name: string): string {
  const value = pkg?.scripts?.[name];
  return typeof value === 'string' ? value : '';
}

function profileGateIds(profiles: any, profile: string): string[] {
  return list(profiles?.profiles?.[profile]?.gate_ids);
}

function requiredArtifacts(manifest: any): string[] {
  return list(manifest?.required_artifacts);
}

function releaseGovernanceArtifacts(manifest: any): string[] {
  return list(manifest?.artifact_groups?.release_governance);
}

function optionalReports(manifest: any): string[] {
  return list(manifest?.optional_reports);
}

function missingTokens(source: string, tokens: string[]): string[] {
  return tokens.filter((token) => !source.includes(token));
}

function writeMarkdown(path: string, checks: Check[], pass: boolean): void {
  ensureParent(path);
  const lines = [
    '# Incident Governance Closure Guard',
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
  const registryPath = arg('registry', REGISTRY);
  const profilesPath = arg('profiles', PROFILES);
  const manifestPath = arg('manifest', MANIFEST);
  const outJson = arg('out-json', OUT_JSON);
  const outMarkdown = arg('out-markdown', OUT_MARKDOWN);
  const strict = flag('strict', true);

  const registry = readJson(registryPath);
  const profiles = readJson(profilesPath);
  const manifest = readJson(manifestPath);
  const pkg = readJson(PACKAGE_JSON);
  const ci = readText(CI_WORKFLOW);
  const policyText = readText(POLICY);
  const processText = readText(PROCESS_DOC);
  const gateText = readText(GATE_SCRIPT);
  const testText = readText(GATE_TEST);
  const policyMissing = missingTokens(policyText, POLICY_TOKENS);
  const processMissing = missingTokens(processText, PROCESS_TOKENS);
  const gateMissing = missingTokens(gateText, GATE_TOKENS);
  const testMissing = missingTokens(testText, TEST_TOKENS);
  const required = requiredArtifacts(manifest);
  const releaseGovernance = releaseGovernanceArtifacts(manifest);
  const reports = optionalReports(manifest);

  const checks: Check[] = [
    check('policy_config_exists', existsSync(POLICY), POLICY),
    check('owner_roster_exists', existsSync(OWNER_ROSTER), OWNER_ROSTER),
    check('waiver_register_exists', existsSync(WAIVERS), WAIVERS),
    check('policy_doc_exists', existsSync(POLICY_DOC), POLICY_DOC),
    check('process_doc_exists', existsSync(PROCESS_DOC), PROCESS_DOC),
    check('incident_gate_script_exists', existsSync(GATE_SCRIPT), GATE_SCRIPT),
    check('incident_gate_test_exists', existsSync(GATE_TEST), GATE_TEST),
    check('policy_required_tokens_present', policyMissing.length === 0, policyMissing.join(', ')),
    check('process_required_tokens_present', processMissing.length === 0, processMissing.join(', ')),
    check('gate_required_tokens_present', gateMissing.length === 0, gateMissing.join(', ')),
    check('test_required_tokens_present', testMissing.length === 0, testMissing.join(', ')),
    check('incident_gate_package_script_present', packageScript(pkg, 'ops:incident-governance:gate').includes(GATE_SCRIPT), packageScript(pkg, 'ops:incident-governance:gate')),
    check('incident_gate_test_package_script_present', packageScript(pkg, 'test:ops:incident-governance:gate').includes(GATE_TEST), packageScript(pkg, 'test:ops:incident-governance:gate')),
    check('closure_guard_package_script_present', packageScript(pkg, CLOSURE_GATE_ID).includes('tooling:run'), packageScript(pkg, CLOSURE_GATE_ID)),
    check('incident_gate_registry_runnable', registryRunnable(registry, GATE_ID), GATE_ID),
    check('incident_gate_registry_exports_json', registryArtifacts(registry, GATE_ID).includes(INCIDENT_JSON), INCIDENT_JSON),
    check('incident_gate_registry_exports_markdown', registryArtifacts(registry, GATE_ID).includes(INCIDENT_MARKDOWN), INCIDENT_MARKDOWN),
    check('closure_guard_registry_runnable', registryRunnable(registry, CLOSURE_GATE_ID), CLOSURE_GATE_ID),
    check('closure_guard_registry_exports_json', registryArtifacts(registry, CLOSURE_GATE_ID).includes(outJson), outJson),
    check('closure_guard_registry_exports_markdown', registryArtifacts(registry, CLOSURE_GATE_ID).includes(outMarkdown), outMarkdown),
    check('fast_profile_incident_gate_covered', profileGateIds(profiles, 'fast').includes(GATE_ID), GATE_ID),
    check('boundary_profile_incident_gate_covered', profileGateIds(profiles, 'boundary').includes(GATE_ID), GATE_ID),
    check('release_profile_incident_gate_covered', profileGateIds(profiles, 'release').includes(GATE_ID), GATE_ID),
    check('fast_profile_closure_guard_covered', profileGateIds(profiles, 'fast').includes(CLOSURE_GATE_ID), CLOSURE_GATE_ID),
    check('boundary_profile_closure_guard_covered', profileGateIds(profiles, 'boundary').includes(CLOSURE_GATE_ID), CLOSURE_GATE_ID),
    check('release_profile_closure_guard_covered', profileGateIds(profiles, 'release').includes(CLOSURE_GATE_ID), CLOSURE_GATE_ID),
    check('incident_gate_artifact_required_in_proof_pack', required.includes(INCIDENT_JSON), INCIDENT_JSON),
    check('closure_guard_artifact_required_in_proof_pack', required.includes(outJson), outJson),
    check('incident_gate_grouped_as_release_governance', releaseGovernance.includes(INCIDENT_JSON), INCIDENT_JSON),
    check('closure_guard_grouped_as_release_governance', releaseGovernance.includes(outJson), outJson),
    check('incident_gate_report_listed', reports.includes(INCIDENT_MARKDOWN), INCIDENT_MARKDOWN),
    check('closure_guard_report_listed', reports.includes(outMarkdown), outMarkdown),
    check('ci_workflow_has_incident_job', ci.includes('incident-governance:') && ci.includes('npm run -s ops:incident-governance:gate'), 'incident-governance'),
    check('ci_workflow_runs_incident_regression_test', ci.includes('npm run -s test:ops:incident-governance:gate'), 'test:ops:incident-governance:gate'),
  ];

  const pass = checks.every((row) => row.ok);
  const payload = {
    ok: pass,
    type: 'incident_governance_closure_guard',
    srs_id: SRS_ID,
    legacy_srs_id: LEGACY_SRS_ID,
    generated_at: new Date().toISOString(),
    inputs: { registry_path: registryPath, profiles_path: profilesPath, manifest_path: manifestPath },
    summary: {
      checks: checks.length,
      passed: checks.filter((row) => row.ok).length,
      failed: checks.filter((row) => !row.ok).length,
    },
    checks,
    artifact_paths: [outJson, outMarkdown, INCIDENT_JSON, INCIDENT_MARKDOWN],
  };

  ensureParent(outJson);
  writeFileSync(outJson, `${JSON.stringify(payload, null, 2)}\n`);
  writeMarkdown(outMarkdown, checks, pass);
  if (strict && !pass) process.exitCode = 1;
}

main();
