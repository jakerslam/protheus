#!/usr/bin/env node
'use strict';

function clean(value: unknown, max = 160): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function envTrue(value: unknown, fallback = false): boolean {
  const raw = clean(value, 24).toLowerCase();
  if (!raw) return fallback;
  if (raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on') return true;
  if (raw === '0' || raw === 'false' || raw === 'no' || raw === 'off') return false;
  return fallback;
}

function releaseChannel(env: NodeJS.ProcessEnv = process.env): string {
  const raw = clean(env.INFRING_RELEASE_CHANNEL || env.INFRING_RELEASE_CHANNEL || '', 48).toLowerCase();
  return raw || 'stable';
}

function isProductionReleaseChannel(channel: string): boolean {
  const normalized = clean(channel, 48).toLowerCase();
  return (
    normalized === 'stable' ||
    normalized === 'production' ||
    normalized === 'prod' ||
    normalized === 'ga' ||
    normalized === 'release'
  );
}

function parseArgs(argv: string[]) {
  let strict = false;
  let json = false;
  for (const raw of argv) {
    const token = clean(raw, 120).toLowerCase();
    if (!token) continue;
    if (token === '--strict' || token === '--strict=1' || token === '--strict=true') strict = true;
    if (token === '--json' || token === '--json=1' || token === '--json=true') json = true;
  }
  return { strict, json };
}

type TopologyViolation = {
  id: string;
  detail: string;
};

function collectTopologyStatus(env: NodeJS.ProcessEnv = process.env) {
  const channel = releaseChannel(env);
  const production = isProductionReleaseChannel(channel);
  const ipcDaemonEnabled = envTrue(
    env.INFRING_OPS_IPC_DAEMON || env.INFRING_OPS_IPC_DAEMON,
    true,
  );
  const ipcStrictEnabled = envTrue(
    env.INFRING_OPS_IPC_STRICT || env.INFRING_OPS_IPC_STRICT,
    true,
  );
  const processFallbackRequested = envTrue(
    env.INFRING_OPS_ALLOW_PROCESS_FALLBACK || env.INFRING_OPS_ALLOW_PROCESS_FALLBACK,
    false,
  );
  const sdkProcessTransportRequested = envTrue(env.INFRING_SDK_ALLOW_PROCESS_TRANSPORT, false);
  const legacyRunnerRequested = envTrue(
    env.INFRING_OPS_FORCE_LEGACY_PROCESS_RUNNER || env.INFRING_OPS_FORCE_LEGACY_PROCESS_RUNNER,
    false,
  );

  const processFallbackEffective = processFallbackRequested && !production;
  const sdkProcessTransportEffective = sdkProcessTransportRequested && !production;
  const legacyRunnerEffective = legacyRunnerRequested && !production;

  const violations: TopologyViolation[] = [];
  if (!ipcDaemonEnabled) {
    violations.push({
      id: 'resident_ipc_disabled',
      detail: 'INFRING_OPS_IPC_DAEMON/INFRING_OPS_IPC_DAEMON must remain enabled',
    });
  }
  if (!ipcStrictEnabled) {
    violations.push({
      id: 'ipc_strict_disabled',
      detail: 'INFRING_OPS_IPC_STRICT/INFRING_OPS_IPC_STRICT must remain enabled',
    });
  }
  if (processFallbackEffective) {
    violations.push({
      id: 'ops_process_fallback_effective',
      detail: 'process fallback is active; resident IPC is no longer authoritative',
    });
  }
  if (sdkProcessTransportEffective) {
    violations.push({
      id: 'sdk_process_transport_effective',
      detail: 'sdk process transport fallback is active',
    });
  }
  if (legacyRunnerEffective) {
    violations.push({
      id: 'legacy_process_runner_effective',
      detail: 'legacy process runner override is active',
    });
  }

  return {
    ok: violations.length === 0,
    type: 'transport_topology_status',
    release_channel: channel,
    production_release: production,
    topology_mode: 'resident_ipc_authoritative',
    transport: {
      ipc_daemon_enabled: ipcDaemonEnabled,
      ipc_strict_enabled: ipcStrictEnabled,
      process_fallback_requested: processFallbackRequested,
      process_fallback_effective: processFallbackEffective,
      sdk_process_transport_requested: sdkProcessTransportRequested,
      sdk_process_transport_effective: sdkProcessTransportEffective,
      legacy_process_runner_requested: legacyRunnerRequested,
      legacy_process_runner_effective: legacyRunnerEffective,
    },
    violations,
  };
}

function run(argv: string[] = process.argv.slice(2), env: NodeJS.ProcessEnv = process.env): number {
  const { strict, json } = parseArgs(argv);
  const payload = collectTopologyStatus(env);
  if (json || strict) {
    process.stdout.write(`${JSON.stringify(payload)}\n`);
  } else {
    process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
  }
  if (strict && !payload.ok) return 1;
  return 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2), process.env));
}

module.exports = {
  run,
  collectTopologyStatus,
  releaseChannel,
  isProductionReleaseChannel,
};
