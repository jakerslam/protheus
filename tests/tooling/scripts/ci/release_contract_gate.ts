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
  const windowsReadmeInstallCommand =
    'Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force; $tmp = Join-Path $env:TEMP "infring-install.ps1"; irm https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.ps1 -OutFile $tmp -ErrorAction Stop; & $tmp -Repair -Full; Remove-Item $tmp -Force -ErrorAction SilentlyContinue';
  const ok =
    installPs.includes('protheus-ops.exe') &&
    installPs.includes('infringd.cmd') &&
    installPs.includes('Install-AllowDirectMsvcBootstrapEnabled') &&
    installPs.includes('INFRING_INSTALL_REPAIR') &&
    installPs.includes('INFRING_INSTALL_FULL') &&
    installPs.includes('Compatibility shim for operators accustomed to `-Force`.') &&
    installPsForceRepairShim &&
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
