#!/usr/bin/env tsx

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname } from 'path';

const SRS_ID = 'V12-INSTALL-RELIABILITY-CLOSURE-001';
const LEGACY_SRS_IDS = ['V11-INSTALL-005', 'V11-INSTALL-008'];
const INSTALL_PS1 = 'install.ps1';
const INSTALL_SH = 'install.sh';
const REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const PROFILES = 'tests/tooling/config/verify_profiles.json';
const MANIFEST = 'tests/tooling/config/release_proof_pack_manifest.json';
const PACKAGE_JSON = 'package.json';
const OUT_JSON = 'core/local/artifacts/installer_reliability_closure_guard_current.json';
const OUT_MARKDOWN = 'local/workspace/reports/INSTALLER_RELIABILITY_CLOSURE_GUARD_CURRENT.md';
const WINDOWS_GATE_ID = 'ops:windows-installer:contract:guard';
const CLOSURE_GATE_ID = 'ops:installer-reliability:closure:guard';
const WINDOWS_CONTRACT_JSON = 'core/local/artifacts/windows_installer_contract_guard_current.json';
const WINDOWS_RELIABILITY_JSON = 'core/local/artifacts/windows_install_reliability_current.json';
const WINDOWS_CONTRACT_MD = 'local/workspace/reports/WINDOWS_INSTALLER_CONTRACT_GUARD_CURRENT.md';
const WINDOWS_RELIABILITY_MD = 'local/workspace/reports/WINDOWS_INSTALL_RELIABILITY_CURRENT.md';

type Check = { id: string; ok: boolean; detail?: string };

const PS1_OFFLINE_TOKENS = [
  '[switch]$Offline',
  '$InstallOffline = $true',
  'Offline install mode requires an explicit release tag',
  'offline cache miss for $Asset',
  'offline cache invalid for $Asset',
  'offline cache read failed for $Asset',
  'checksum manifest: $candidate (cache)',
  'offline_asset_cache_miss',
  'mode: offline (network disabled; cached artifacts only)',
  'asset_cache_enabled = [bool]$script:InstallAssetCache',
];

const SH_OFFLINE_TOKENS = [
  '--offline)',
  'offline mode requires an explicit release tag',
  'offline cache miss for $asset_name',
  'offline cache invalid for $asset_name',
  'checksum_manifest: ${checksum_asset} (cache)',
  'mode: offline (network disabled; using cached artifacts only)',
  'install_mode_offline:',
];

const RELEASE_TAG_TOKENS = [
  'tag_state_missing',
  'release_tag_changed',
  'runtime_missing',
  'repair_mode',
  'Workspace runtime refresh required but not applied',
  'workspace_release_tag_written:',
  'workspace_release_tag_write_verified:',
  'Assert-WorkspaceRuntimeReleaseTagState',
];

const WRAPPER_TOKENS = [
  'Ensure-RepairBootstrapWrapperFloor',
  'Write-PowerShellShim -Path $infringPs1 -TargetCmd "infring.cmd"',
  'Set-Content -LiteralPath $cmdPath -Value $cmdContent -Encoding ASCII -Force',
  'Set-Content -LiteralPath $ps1Path -Value $psContent -Encoding UTF8 -Force',
  'repair wrapper floor failed; missing wrappers',
  '$target = Join-Path $PSScriptRoot "__TARGET__"',
];

const DOCS = [
  'README.md',
  'docs/client/GETTING_STARTED.md',
  'docs/workspace/manuals/infring_manual_help_tab.md',
];

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

function missingTokens(source: string, tokens: string[]): string[] {
  return tokens.filter((token) => !source.includes(token));
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

function writeMarkdown(path: string, checks: Check[], pass: boolean): void {
  ensureParent(path);
  const lines = [
    '# Installer Reliability Closure Guard',
    '',
    `- pass: ${pass}`,
    `- srs_id: ${SRS_ID}`,
    `- legacy_srs_ids: ${LEGACY_SRS_IDS.join(', ')}`,
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
  const ps1 = readText(INSTALL_PS1);
  const sh = readText(INSTALL_SH);
  const required = requiredArtifacts(manifest);
  const releaseGovernance = releaseGovernanceArtifacts(manifest);
  const reports = optionalReports(manifest);
  const docsText = DOCS.map((path) => `${path}\n${readText(path)}`).join('\n---\n');

  const missingPs1Offline = missingTokens(ps1, PS1_OFFLINE_TOKENS);
  const missingShOffline = missingTokens(sh, SH_OFFLINE_TOKENS);
  const missingReleaseTagPs1 = missingTokens(ps1, RELEASE_TAG_TOKENS);
  const missingWrapper = missingTokens(ps1, WRAPPER_TOKENS);
  const missingDocs = DOCS.filter((path) => {
    const raw = readText(path);
    return !raw.includes('Optional offline/cached reinstall (PowerShell)') || !raw.includes('-Offline -ReleaseVersion');
  });

  const checks: Check[] = [
    check('windows_contract_gate_package_script_present', packageScript(pkg, WINDOWS_GATE_ID).includes('windows_installer_contract_guard.ts'), packageScript(pkg, WINDOWS_GATE_ID)),
    check('closure_gate_package_script_present', packageScript(pkg, CLOSURE_GATE_ID).includes('tooling:run'), packageScript(pkg, CLOSURE_GATE_ID)),
    check('windows_contract_gate_registry_runnable', registryRunnable(registry, WINDOWS_GATE_ID), WINDOWS_GATE_ID),
    check('closure_gate_registry_runnable', registryRunnable(registry, CLOSURE_GATE_ID), CLOSURE_GATE_ID),
    check('windows_contract_artifacts_registered', registryArtifacts(registry, WINDOWS_GATE_ID).includes(WINDOWS_CONTRACT_JSON) && registryArtifacts(registry, WINDOWS_GATE_ID).includes(WINDOWS_RELIABILITY_JSON), registryArtifacts(registry, WINDOWS_GATE_ID).join(', ')),
    check('closure_artifacts_registered', registryArtifacts(registry, CLOSURE_GATE_ID).includes(outJson) && registryArtifacts(registry, CLOSURE_GATE_ID).includes(outMarkdown), registryArtifacts(registry, CLOSURE_GATE_ID).join(', ')),
    check('ps1_offline_cache_contract_tokens_present', missingPs1Offline.length === 0, missingPs1Offline.join(', ')),
    check('sh_offline_cache_contract_tokens_present', missingShOffline.length === 0, missingShOffline.join(', ')),
    check('docs_offline_cache_examples_present', missingDocs.length === 0 && docsText.includes('INFRING_INSTALL_OFFLINE'), missingDocs.join(', ')),
    check('ps1_release_tag_refresh_contract_tokens_present', missingReleaseTagPs1.length === 0, missingReleaseTagPs1.join(', ')),
    check('ps1_repair_wrapper_floor_contract_tokens_present', missingWrapper.length === 0, missingWrapper.join(', ')),
    check('sh_release_tag_readback_contract_present', sh.includes('workspace_release_tag_matches "$WORKSPACE_DIR" "$version"') && sh.includes('workspace_release_tag_write_verified=1'), 'install.sh readback verification'),
    check('windows_contract_artifacts_required', required.includes(WINDOWS_CONTRACT_JSON) && required.includes(WINDOWS_RELIABILITY_JSON), `${WINDOWS_CONTRACT_JSON}, ${WINDOWS_RELIABILITY_JSON}`),
    check('closure_artifact_required', required.includes(outJson), outJson),
    check('windows_contract_artifacts_release_governance_grouped', releaseGovernance.includes(WINDOWS_CONTRACT_JSON) && releaseGovernance.includes(WINDOWS_RELIABILITY_JSON), `${WINDOWS_CONTRACT_JSON}, ${WINDOWS_RELIABILITY_JSON}`),
    check('closure_artifact_release_governance_grouped', releaseGovernance.includes(outJson), outJson),
    check('windows_reports_listed', reports.includes(WINDOWS_CONTRACT_MD) && reports.includes(WINDOWS_RELIABILITY_MD), `${WINDOWS_CONTRACT_MD}, ${WINDOWS_RELIABILITY_MD}`),
    check('closure_report_listed', reports.includes(outMarkdown), outMarkdown),
    check('fast_profile_windows_gate_covered', profileGateIds(profiles, 'fast').includes(WINDOWS_GATE_ID), WINDOWS_GATE_ID),
    check('fast_profile_closure_gate_covered', profileGateIds(profiles, 'fast').includes(CLOSURE_GATE_ID), CLOSURE_GATE_ID),
    check('boundary_profile_windows_gate_covered', profileGateIds(profiles, 'boundary').includes(WINDOWS_GATE_ID), WINDOWS_GATE_ID),
    check('boundary_profile_closure_gate_covered', profileGateIds(profiles, 'boundary').includes(CLOSURE_GATE_ID), CLOSURE_GATE_ID),
    check('release_profile_windows_gate_covered', profileGateIds(profiles, 'release').includes(WINDOWS_GATE_ID), WINDOWS_GATE_ID),
    check('release_profile_closure_gate_covered', profileGateIds(profiles, 'release').includes(CLOSURE_GATE_ID), CLOSURE_GATE_ID),
  ];

  const pass = checks.every((row) => row.ok);
  const payload = {
    ok: pass,
    type: 'installer_reliability_closure_guard',
    srs_id: SRS_ID,
    legacy_srs_ids: LEGACY_SRS_IDS,
    generated_at: new Date().toISOString(),
    inputs: { registry_path: registryPath, profiles_path: profilesPath, manifest_path: manifestPath },
    summary: {
      checks: checks.length,
      passed: checks.filter((row) => row.ok).length,
      failed: checks.filter((row) => !row.ok).length,
    },
    checks,
    artifact_paths: [outJson, outMarkdown, WINDOWS_CONTRACT_JSON, WINDOWS_RELIABILITY_JSON],
  };

  ensureParent(outJson);
  writeFileSync(outJson, `${JSON.stringify(payload, null, 2)}\n`);
  writeMarkdown(outMarkdown, checks, pass);
  if (strict && !pass) process.exitCode = 1;
}

main();
