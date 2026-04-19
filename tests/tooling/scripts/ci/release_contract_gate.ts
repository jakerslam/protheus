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
    'asset_probe=',
    'attempted_assets=',
    'source_fallback_attempted=',
    'source_fallback_reason=',
    'preflight_no_reachable_prebuilt_with_missing_msvc=',
    'source_fallback_plan=',
    'main_last_resort_fallback=',
    'toolchain:cargo=',
    'auto_bootstrap:auto_rustup=',
    'auto_bootstrap:direct_msvc=',
    'install_policy:allow_no_msvc_source_fallback=',
    'compatible_release_fallback=',
    'pinned_version_compatible_fallback=',
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
  return [
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

function buildReport() {
  const checks: Check[] = [
    runGateCheck('runtime_dependency_contract'),
    runGateCheck('ops:legacy-runner:release-guard'),
    runGateCheck('ops:transport:spawn-audit'),
    runGateCheck('release_policy_gate'),
    runGateCheck('ops:assimilation:v1:support:guard'),
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
  const report = buildReport();
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
