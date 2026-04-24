#!/usr/bin/env tsx

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname } from 'path';

const SRS_ID = 'V12-DX-EFFECTIVE-LOC-CLOSURE-001';
const LEGACY_SRS_ID = 'V11-DX-004';
const MANIFEST = 'tests/tooling/config/release_proof_pack_manifest.json';
const REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const PACKAGE_JSON = 'package.json';
const METRIC_RUNNER = 'tests/tooling/scripts/ci/effective_loc_metric.ts';
const OUT_JSON = 'core/local/artifacts/effective_loc_metric_contract_guard_current.json';
const OUT_MARKDOWN = 'local/workspace/reports/EFFECTIVE_LOC_METRIC_CONTRACT_GUARD_CURRENT.md';
const METRIC_JSON = 'core/local/artifacts/effective_loc_metric_current.json';
const METRIC_MARKDOWN = 'local/workspace/reports/EFFECTIVE_LOC_METRIC_CURRENT.md';
const CONTRACT_GATE_ID = 'ops:effective-loc:contract:guard';
const METRIC_GATE_ID = 'ops:effective-loc:metric';
const METRIC_ID = 'effective_production_nonblank_loc_v1';

const REQUIRED_RUNNER_TOKENS = [
  METRIC_ID,
  "const DEFAULT_OUT = 'core/local/artifacts/effective_loc_metric_current.json'",
  "const DEFAULT_MD = 'local/workspace/reports/EFFECTIVE_LOC_METRIC_CURRENT.md'",
  "'*.rs'",
  "'*.ts'",
  "'*.tsx'",
  ":(exclude)docs/**",
  ":(exclude)local/**",
  ":(exclude)tests/**",
  ":(exclude)**/vendor/**",
  ":(exclude)**/*.min.ts",
  ":(exclude)**/*.d.ts",
  'trackedFilesAtRef',
  'ls-tree',
  'git grep',
  'include_tracked_only: true',
  'base_ref',
  'delta_nonblank_loc',
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

function registryCommandText(registry: any, gateId: string): string {
  const entry = registryGate(registry, gateId);
  if (Array.isArray(entry?.command)) return entry.command.join(' ');
  if (typeof entry?.script === 'string') return entry.script;
  return '';
}

function registryArtifacts(registry: any, gateId: string): string[] {
  return list(registryGate(registry, gateId)?.artifact_paths);
}

function requiredArtifacts(manifest: any): string[] {
  return list(manifest?.required_artifacts);
}

function workloadArtifacts(manifest: any): string[] {
  return list(manifest?.artifact_groups?.workload_and_quality);
}

function optionalReports(manifest: any): string[] {
  return list(manifest?.optional_reports);
}

function packageScript(pkg: any, name: string): string {
  const value = pkg?.scripts?.[name];
  return typeof value === 'string' ? value : '';
}

function verifyProfileGateIds(profiles: any, profile: string): string[] {
  return list(profiles?.[profile]?.gate_ids);
}

function missingTokens(source: string, tokens: string[]): string[] {
  return tokens.filter((token) => !source.includes(token));
}

function writeMarkdown(path: string, checks: Check[], pass: boolean): void {
  ensureParent(path);
  const lines = [
    '# Effective LoC Metric Contract Guard',
    '',
    `- pass: ${pass}`,
    `- srs_id: ${SRS_ID}`,
    `- legacy_srs_id: ${LEGACY_SRS_ID}`,
    `- metric_id: ${METRIC_ID}`,
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
  const packagePath = arg('package', PACKAGE_JSON);
  const runnerPath = arg('runner', METRIC_RUNNER);
  const outJson = arg('out-json', OUT_JSON);
  const outMarkdown = arg('out-markdown', OUT_MARKDOWN);
  const strict = flag('strict', true);

  const manifest = readJson(manifestPath);
  const registry = readJson(registryPath);
  const pkg = readJson(packagePath);
  const runner = readText(runnerPath);
  const required = requiredArtifacts(manifest);
  const workload = workloadArtifacts(manifest);
  const reports = optionalReports(manifest);
  const fastProfile = verifyProfileGateIds(readJson('tests/tooling/config/verify_profiles.json'), 'fast');
  const boundaryProfile = verifyProfileGateIds(readJson('tests/tooling/config/verify_profiles.json'), 'boundary');
  const releaseProfile = verifyProfileGateIds(readJson('tests/tooling/config/verify_profiles.json'), 'release');
  const metricScript = packageScript(pkg, 'metrics:effective-loc');
  const deltaScript = packageScript(pkg, 'metrics:effective-loc:delta-main');
  const metricGateCommand = registryCommandText(registry, METRIC_GATE_ID);
  const contractGateCommand = registryCommandText(registry, CONTRACT_GATE_ID);

  const missingRunnerTokens = missingTokens(runner, REQUIRED_RUNNER_TOKENS);
  const checks: Check[] = [
    check('metric_runner_exists', existsSync(runnerPath), runnerPath),
    check('metric_runner_policy_tokens_present', missingRunnerTokens.length === 0, missingRunnerTokens.join(', ')),
    check('metric_script_registered', metricScript.includes(METRIC_RUNNER) && metricScript.includes('--strict=1'), metricScript),
    check('metric_delta_script_registered', deltaScript.includes(METRIC_RUNNER) && deltaScript.includes('--base-ref=origin/main'), deltaScript),
    check('metric_gate_registered', registryRunnable(registry, METRIC_GATE_ID), METRIC_GATE_ID),
    check('metric_gate_runs_metric_runner', metricGateCommand.includes(METRIC_RUNNER) && metricGateCommand.includes('--strict=1'), metricGateCommand),
    check('metric_gate_exports_json', registryArtifacts(registry, METRIC_GATE_ID).includes(METRIC_JSON), METRIC_JSON),
    check('metric_gate_exports_markdown', registryArtifacts(registry, METRIC_GATE_ID).includes(METRIC_MARKDOWN), METRIC_MARKDOWN),
    check('contract_guard_registered', registryRunnable(registry, CONTRACT_GATE_ID), CONTRACT_GATE_ID),
    check('contract_guard_runs_this_file', contractGateCommand.includes('effective_loc_metric_contract_guard.ts'), contractGateCommand),
    check('contract_guard_exports_json', registryArtifacts(registry, CONTRACT_GATE_ID).includes(outJson), outJson),
    check('contract_guard_exports_markdown', registryArtifacts(registry, CONTRACT_GATE_ID).includes(outMarkdown), outMarkdown),
    check('metric_artifact_required_in_proof_pack', required.includes(METRIC_JSON), METRIC_JSON),
    check('contract_artifact_required_in_proof_pack', required.includes(OUT_JSON), OUT_JSON),
    check('metric_artifact_grouped_as_workload_quality', workload.includes(METRIC_JSON), METRIC_JSON),
    check('contract_artifact_grouped_as_workload_quality', workload.includes(OUT_JSON), OUT_JSON),
    check('metric_report_listed', reports.includes(METRIC_MARKDOWN), METRIC_MARKDOWN),
    check('contract_report_listed', reports.includes(OUT_MARKDOWN), OUT_MARKDOWN),
    check('contract_guard_fast_profile_covered', fastProfile.includes(CONTRACT_GATE_ID), CONTRACT_GATE_ID),
    check('contract_guard_boundary_profile_covered', boundaryProfile.includes(CONTRACT_GATE_ID), CONTRACT_GATE_ID),
    check('metric_release_profile_covered', releaseProfile.includes(METRIC_GATE_ID), METRIC_GATE_ID),
    check('contract_guard_release_profile_covered', releaseProfile.includes(CONTRACT_GATE_ID), CONTRACT_GATE_ID),
  ];

  const pass = checks.every((row) => row.ok);
  const payload = {
    ok: pass,
    type: 'effective_loc_metric_contract_guard',
    srs_id: SRS_ID,
    legacy_srs_id: LEGACY_SRS_ID,
    metric_id: METRIC_ID,
    generated_at: new Date().toISOString(),
    inputs: {
      manifest_path: manifestPath,
      registry_path: registryPath,
      package_path: packagePath,
      metric_runner_path: runnerPath,
    },
    summary: {
      checks: checks.length,
      passed: checks.filter((row) => row.ok).length,
      failed: checks.filter((row) => !row.ok).length,
    },
    checks,
    artifact_paths: [outJson, outMarkdown, METRIC_JSON, METRIC_MARKDOWN],
  };

  ensureParent(outJson);
  writeFileSync(outJson, `${JSON.stringify(payload, null, 2)}\n`);
  writeMarkdown(outMarkdown, checks, pass);
  if (strict && !pass) process.exitCode = 1;
}

main();
