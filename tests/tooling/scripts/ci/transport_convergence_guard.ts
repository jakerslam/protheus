#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

type Args = {
  strict: boolean;
  out: string;
};

type Violation = {
  file: string;
  reason: string;
  detail: string;
};

const ROOT = process.cwd();

function parseArgs(argv: string[]): Args {
  const args: Args = {
    strict: false,
    out: 'core/local/artifacts/transport_convergence_guard_current.json',
  };
  for (const token of argv) {
    if (token === '--strict') args.strict = true;
    else if (token.startsWith('--strict=')) {
      const value = token.slice('--strict='.length).toLowerCase();
      args.strict = value === '1' || value === 'true' || value === 'yes' || value === 'on';
    } else if (token.startsWith('--out=')) {
      args.out = token.slice('--out='.length);
    }
  }
  return args;
}

function rel(filePath: string): string {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function walk(base: string, exts: Set<string>): string[] {
  if (!fs.existsSync(base)) return [];
  const out: string[] = [];
  const stack = [base];
  while (stack.length > 0) {
    const current = stack.pop() as string;
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const abs = path.join(current, entry.name);
      const rp = rel(abs);
      if (rp.includes('/node_modules/') || rp.includes('/dist/') || rp.includes('/target/')) {
        continue;
      }
      if (entry.isDirectory()) {
        stack.push(abs);
      } else if (entry.isFile() && exts.has(path.extname(entry.name))) {
        out.push(abs);
      }
    }
  }
  return out.sort();
}

function countToken(source: string, token: string): number {
  if (!source) return 0;
  return source.split(token).length - 1;
}

function run(args: Args): number {
  const violations: Violation[] = [];

  const adaptersRuntimeFiles = walk(path.join(ROOT, 'adapters', 'runtime'), new Set(['.ts', '.tsx']));
  const orchestrationScriptFiles = walk(
    path.join(ROOT, 'surface', 'orchestration', 'scripts'),
    new Set(['.ts', '.tsx'])
  );
  const sdkTransportFiles = walk(path.join(ROOT, 'packages', 'infring-sdk', 'src'), new Set(['.ts', '.tsx']));

  const allowedSpawnSyncByFile = new Map<string, number>([
    ['adapters/runtime/dev_only/legacy_process_runner.ts', 1],
    ['adapters/runtime/dev_only/ops_lane_process_fallback.ts', 1],
    ['adapters/runtime/dashboard_asset_router.ts', 1],
  ]);

  for (const file of adaptersRuntimeFiles) {
    const source = fs.readFileSync(file, 'utf8');
    const relPath = rel(file);
    const spawnSyncCount = countToken(source, 'spawnSync(');
    const allowed = allowedSpawnSyncByFile.get(relPath);
    if (typeof allowed === 'number') {
      if (spawnSyncCount > allowed) {
        violations.push({
          file: relPath,
          reason: 'spawn_sync_budget_exceeded',
          detail: `spawnSync count ${spawnSyncCount} > allowed ${allowed}`,
        });
      }
      continue;
    }
    if (spawnSyncCount > 0) {
      violations.push({
        file: relPath,
        reason: 'spawn_sync_outside_adapter_allowlist_forbidden',
        detail: `spawnSync count ${spawnSyncCount}`,
      });
    }
  }

  for (const file of orchestrationScriptFiles) {
    const source = fs.readFileSync(file, 'utf8');
    const relPath = rel(file);
    const spawnSyncCount = countToken(source, 'spawnSync(');
    const spawnCount = countToken(source, 'spawn(');
    if (spawnSyncCount > 0) {
      violations.push({
        file: relPath,
        reason: 'orchestration_scripts_spawn_sync_forbidden',
        detail: `spawnSync count ${spawnSyncCount}`,
      });
    }
    if (spawnCount > 0) {
      violations.push({
        file: relPath,
        reason: 'orchestration_scripts_direct_spawn_forbidden',
        detail: `spawn count ${spawnCount}`,
      });
    }
  }

  for (const file of sdkTransportFiles) {
    const source = fs.readFileSync(file, 'utf8');
    const relPath = rel(file);
    const spawnSyncCount = countToken(source, 'spawnSync(');
    if (spawnSyncCount > 0) {
      violations.push({
        file: relPath,
        reason: 'sdk_spawn_sync_forbidden',
        detail: `spawnSync count ${spawnSyncCount}`,
      });
    }
  }

  const opsLaneBridgePath = path.join(ROOT, 'adapters', 'runtime', 'ops_lane_bridge.ts');
  if (fs.existsSync(opsLaneBridgePath)) {
    const source = fs.readFileSync(opsLaneBridgePath, 'utf8');
    if (!source.includes('ipcBridgeEnabled()')) {
      violations.push({
        file: rel(opsLaneBridgePath),
        reason: 'ops_lane_bridge_ipc_guard_missing',
        detail: 'expected ipcBridgeEnabled() transport gate',
      });
    }
    if (!source.includes('processFallbackEnabled()')) {
      violations.push({
        file: rel(opsLaneBridgePath),
        reason: 'ops_lane_bridge_process_fallback_gate_missing',
        detail: 'expected processFallbackEnabled() fallback gate',
      });
    }
  }

  const runInfringOpsPath = path.join(ROOT, 'adapters', 'runtime', 'run_infring_ops.ts');
  if (fs.existsSync(runInfringOpsPath)) {
    const source = fs.readFileSync(runInfringOpsPath, 'utf8');
    if (!source.includes('createOpsLaneBridge')) {
      violations.push({
        file: rel(runInfringOpsPath),
        reason: 'run_infring_ops_bridge_first_contract_missing',
        detail: 'expected createOpsLaneBridge import/use for resident-first transport',
      });
    }
    if (!source.includes('preferLocalCore: true')) {
      violations.push({
        file: rel(runInfringOpsPath),
        reason: 'run_infring_ops_prefer_local_core_missing',
        detail: 'expected preferLocalCore: true for bridge-first path',
      });
    }
    if (!source.includes('INFRING_OPS_FORCE_LEGACY_PROCESS_RUNNER')) {
      violations.push({
        file: rel(runInfringOpsPath),
        reason: 'run_infring_ops_legacy_escape_hatch_missing',
        detail: 'expected explicit legacy process runner override env',
      });
    }
    if (!source.includes('isProductionReleaseChannel')) {
      violations.push({
        file: rel(runInfringOpsPath),
        reason: 'run_infring_ops_production_channel_guard_missing',
        detail: 'expected release-channel lock for legacy process runner overrides',
      });
    }
    if (!source.includes('process_fallback_forbidden_in_production')) {
      violations.push({
        file: rel(runInfringOpsPath),
        reason: 'run_infring_ops_production_fallback_lock_missing',
        detail: 'expected explicit production fallback lock marker for bridge path',
      });
    }
    if (!source.includes('INFRING_OPS_PROCESS_FALLBACK_POLICY_REASON')) {
      violations.push({
        file: rel(runInfringOpsPath),
        reason: 'run_infring_ops_production_fallback_reason_signal_missing',
        detail: 'expected policy reason env signal when forcing fallback off in production',
      });
    }
    if (!source.includes('isProductionReleaseChannel(releaseChannel(process.env))')) {
      violations.push({
        file: rel(runInfringOpsPath),
        reason: 'run_infring_ops_bridge_path_release_guard_missing',
        detail: 'expected explicit production release-channel guard in bridge path',
      });
    }
    if (!source.includes("envOverrides.INFRING_OPS_ALLOW_PROCESS_FALLBACK = '0'")) {
      violations.push({
        file: rel(runInfringOpsPath),
        reason: 'run_infring_ops_bridge_infring_process_fallback_zero_missing',
        detail: 'expected INFRING_OPS_ALLOW_PROCESS_FALLBACK forced to 0 in production bridge path',
      });
    }
    if (!source.includes("envOverrides.INFRING_SDK_ALLOW_PROCESS_TRANSPORT = '0'")) {
      violations.push({
        file: rel(runInfringOpsPath),
        reason: 'run_infring_ops_bridge_sdk_process_transport_zero_missing',
        detail: 'expected INFRING_SDK_ALLOW_PROCESS_TRANSPORT forced to 0 in production bridge path',
      });
    }
    if (source.includes('spawnSync(')) {
      violations.push({
        file: rel(runInfringOpsPath),
        reason: 'run_infring_ops_spawn_sync_entrypoint_forbidden',
        detail: 'release-path entrypoint must not embed spawnSync legacy runner logic',
      });
    }
    if (!source.includes("./dev_only/legacy_process_runner.ts")) {
      violations.push({
        file: rel(runInfringOpsPath),
        reason: 'run_infring_ops_dev_only_legacy_runner_missing',
        detail: 'expected legacy process runner to be quarantined under adapters/runtime/dev_only',
      });
    }
  }

  const sdkTransportPath = path.join(ROOT, 'packages', 'infring-sdk', 'src', 'transports.ts');
  if (fs.existsSync(sdkTransportPath)) {
    const source = fs.readFileSync(sdkTransportPath, 'utf8');
    if (!source.includes('resident_ipc_authoritative')) {
      violations.push({
        file: rel(sdkTransportPath),
        reason: 'sdk_production_transport_lock_missing',
        detail: 'expected resident IPC authoritative topology marker',
      });
    }
    if (!source.includes('createResidentIpcTransport')) {
      violations.push({
        file: rel(sdkTransportPath),
        reason: 'sdk_release_channel_guard_missing',
        detail: 'expected production SDK surface to expose resident IPC transport only',
      });
    }
    if (source.includes('spawn(') || source.includes('spawnSync(')) {
      violations.push({
        file: rel(sdkTransportPath),
        reason: 'sdk_spawn_path_forbidden',
        detail: 'production SDK transport surface must not shell out',
      });
    }
  }

  const sdkCliDevOnlyPath = path.join(ROOT, 'packages', 'infring-sdk', 'src', 'transports', 'cli_dev_only.ts');
  if (fs.existsSync(sdkCliDevOnlyPath)) {
    const source = fs.readFileSync(sdkCliDevOnlyPath, 'utf8');
    if (!source.includes('process_transport_forbidden_in_production')) {
      violations.push({
        file: rel(sdkCliDevOnlyPath),
        reason: 'sdk_dev_only_transport_lock_missing',
        detail: 'expected dev-only CLI transport to keep production lockout marker',
      });
    }
    if (!source.includes('isProductionReleaseChannel')) {
      violations.push({
        file: rel(sdkCliDevOnlyPath),
        reason: 'sdk_dev_only_release_guard_missing',
        detail: 'expected release-channel policy check in quarantined CLI transport',
      });
    }
  }

  if (fs.existsSync(opsLaneBridgePath)) {
    const source = fs.readFileSync(opsLaneBridgePath, 'utf8');
    if (!source.includes('process_fallback_forbidden_in_production')) {
      violations.push({
        file: rel(opsLaneBridgePath),
        reason: 'ops_lane_bridge_production_fallback_lock_missing',
        detail: 'expected production lockout marker for process fallback',
      });
    }
    if (!source.includes('processFallbackPolicy')) {
      violations.push({
        file: rel(opsLaneBridgePath),
        reason: 'ops_lane_bridge_fallback_policy_helper_missing',
        detail: 'expected centralized process fallback policy helper',
      });
    }
    if (source.includes('spawnSync(')) {
      violations.push({
        file: rel(opsLaneBridgePath),
        reason: 'ops_lane_bridge_spawn_sync_entrypoint_forbidden',
        detail: 'release-path bridge entrypoint must not embed process fallback spawnSync logic',
      });
    }
    if (!source.includes("./dev_only/ops_lane_process_fallback.ts")) {
      violations.push({
        file: rel(opsLaneBridgePath),
        reason: 'ops_lane_bridge_dev_only_process_fallback_missing',
        detail: 'expected process fallback to be quarantined under adapters/runtime/dev_only',
      });
    }
  }

  const legacyDevOnlyPath = path.join(ROOT, 'adapters', 'runtime', 'dev_only', 'legacy_process_runner.ts');
  if (!fs.existsSync(legacyDevOnlyPath)) {
    violations.push({
      file: 'adapters/runtime/dev_only/legacy_process_runner.ts',
      reason: 'legacy_process_runner_dev_only_missing',
      detail: 'expected quarantined legacy runner helper',
    });
  } else if (!fs.readFileSync(legacyDevOnlyPath, 'utf8').includes('legacy_process_runner_dev_only')) {
    violations.push({
      file: rel(legacyDevOnlyPath),
      reason: 'legacy_process_runner_dev_only_marker_missing',
      detail: 'expected explicit dev-only marker',
    });
  }

  const processFallbackDevOnlyPath = path.join(ROOT, 'adapters', 'runtime', 'dev_only', 'ops_lane_process_fallback.ts');
  if (!fs.existsSync(processFallbackDevOnlyPath)) {
    violations.push({
      file: 'adapters/runtime/dev_only/ops_lane_process_fallback.ts',
      reason: 'ops_lane_process_fallback_dev_only_missing',
      detail: 'expected quarantined ops lane process fallback helper',
    });
  } else if (!fs.readFileSync(processFallbackDevOnlyPath, 'utf8').includes('process_fallback_dev_only')) {
    violations.push({
      file: rel(processFallbackDevOnlyPath),
      reason: 'ops_lane_process_fallback_dev_only_marker_missing',
      detail: 'expected explicit dev-only marker',
    });
  }

  const topologyStatusPath = path.join(ROOT, 'client', 'runtime', 'systems', 'ops', 'transport_topology_status.ts');
  if (!fs.existsSync(topologyStatusPath)) {
    violations.push({
      file: 'client/runtime/systems/ops/transport_topology_status.ts',
      reason: 'transport_topology_status_missing',
      detail: 'expected runtime topology self-check entrypoint',
    });
  } else {
    const source = fs.readFileSync(topologyStatusPath, 'utf8');
    if (!source.includes('resident_ipc_authoritative')) {
      violations.push({
        file: rel(topologyStatusPath),
        reason: 'transport_topology_mode_marker_missing',
        detail: 'expected resident IPC topology mode marker',
      });
    }
    if (!source.includes('process_fallback_effective')) {
      violations.push({
        file: rel(topologyStatusPath),
        reason: 'transport_topology_effective_fallback_signal_missing',
        detail: 'expected explicit effective fallback signal in topology report',
      });
    }
  }

  const report = {
    type: 'transport_convergence_guard',
    generated_at: new Date().toISOString(),
    strict: args.strict,
    summary: {
      violation_count: violations.length,
      pass: violations.length === 0,
      scanned: {
        adapters_runtime_files: adaptersRuntimeFiles.length,
        orchestration_script_files: orchestrationScriptFiles.length,
        sdk_transport_files: sdkTransportFiles.length,
      },
    },
    violations,
  };

  const outPath = path.resolve(ROOT, args.out);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report, null, 2));

  if (args.strict && violations.length > 0) return 1;
  return 0;
}

const code = run(parseArgs(process.argv.slice(2)));
process.exit(code);
