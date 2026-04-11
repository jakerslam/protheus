#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';
import { invokeTsModuleSync } from '../../../../client/runtime/lib/in_process_ts_delegate.ts';
import { collectTopologyStatus } from '../../../../client/runtime/systems/ops/transport_topology_status.ts';

type Check = {
  id: string;
  ok: boolean;
  detail: string;
};

const ROOT = process.cwd();
const DEFAULT_OUT = path.join(ROOT, 'core/local/artifacts/release_contract_gate_current.json');
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

function runTsCheck(id: string, scriptRelPath: string, args: string[] = []): Check {
  const out = invokeTsModuleSync(path.join(ROOT, scriptRelPath), {
    argv: args,
    cwd: ROOT,
    exportName: 'run',
    teeStdout: false,
    teeStderr: false,
  });
  const status = Number.isFinite(Number(out.status)) ? Number(out.status) : 1;
  const stdout = String(out.stdout || '').trim();
  const stderr = String(out.stderr || '').trim();
  return {
    id,
    ok: status === 0,
    detail: status === 0 ? 'ok' : `status=${status}; stderr=${stderr || stdout}`.slice(0, 500),
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
  const readme = read('README.md');
  const gettingStarted = read('docs/client/GETTING_STARTED.md');
  const ok =
    installPs.includes('protheus-ops.exe') &&
    installPs.includes('infringd.cmd') &&
    /& \$tmp(?:\s+-Repair)?\s+-Full/.test(readme) &&
    readme.includes('install.ps1 -OutFile $tmp') &&
    gettingStarted.includes('install.ps1') &&
    gettingStarted.includes('infring --help');
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
  const bridge = read('adapters/runtime/ops_lane_bridge.ts');
  const runner = read('adapters/runtime/run_protheus_ops.ts');
  const ok =
    sdk.includes('process_transport_forbidden_in_production') &&
    sdk.includes('isProductionReleaseChannel') &&
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

function topologyModeChecks(): Check[] {
  const dev = collectTopologyStatus({
    ...process.env,
    INFRING_RELEASE_CHANNEL: 'dev',
    INFRING_OPS_ALLOW_PROCESS_FALLBACK: '1',
  });
  const stable = collectTopologyStatus({
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
    runTsCheck('runtime_dependency_contract', 'tests/tooling/scripts/ci/runtime_dependency_contract_gate.ts', ['--strict=1']),
    runTsCheck('legacy_runner_release_guard', 'tests/tooling/scripts/ci/legacy_process_runner_release_guard.ts', [
      '--strict=1',
      '--out=core/local/artifacts/legacy_process_runner_release_guard_current.json',
    ]),
    runTsCheck('transport_spawn_audit', 'tests/tooling/scripts/ci/transport_spawn_audit.ts', [
      '--strict=1',
      '--out=core/local/artifacts/transport_spawn_audit_current.json',
    ]),
    runTsCheck('release_policy_gate', 'tests/tooling/scripts/ci/release_policy_gate.ts', [
      '--strict=1',
      '--out=core/local/artifacts/release_policy_gate_current.json',
    ]),
    runTsCheck('assimilation_v1_support_guard', 'tests/tooling/scripts/ci/assimilation_v1_support_guard.ts', [
      '--strict=1',
      '--out=core/local/artifacts/assimilation_v1_support_guard_current.json',
    ]),
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
