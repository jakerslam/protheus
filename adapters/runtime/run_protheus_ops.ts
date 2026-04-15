#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/runtime (shared app bridge helper)

const fs = require('fs');
const path = require('path');
const { createOpsLaneBridge } = require('./ops_lane_bridge.ts');
const {
  resolveBinary: resolveLegacyBinary,
  runLegacyProcessRunner,
} = require('./dev_only/legacy_process_runner.ts');

const ROOT = path.resolve(__dirname, '..', '..');

function envTrue(value) {
  const raw = String(value || '').trim().toLowerCase();
  return raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on';
}

function releaseChannel(env = process.env) {
  const raw = String((env && (env.INFRING_RELEASE_CHANNEL || env.PROTHEUS_RELEASE_CHANNEL)) || '')
    .trim()
    .toLowerCase();
  return raw || 'stable';
}

function isProductionReleaseChannel(channel) {
  const normalized = String(channel || '').trim().toLowerCase();
  return (
    normalized === 'stable' ||
    normalized === 'production' ||
    normalized === 'prod' ||
    normalized === 'ga' ||
    normalized === 'release'
  );
}

function withScopedEnv(overrides, fn) {
  const keys = Object.keys(overrides || {});
  if (keys.length === 0) {
    return fn();
  }
  const previous = {};
  for (const key of keys) {
    previous[key] = Object.prototype.hasOwnProperty.call(process.env, key)
      ? process.env[key]
      : undefined;
    const value = overrides[key];
    if (value === undefined || value === null || value === '') {
      delete process.env[key];
    } else {
      process.env[key] = String(value);
    }
  }
  try {
    return fn();
  } finally {
    for (const key of keys) {
      const value = previous[key];
      if (value === undefined) delete process.env[key];
      else process.env[key] = value;
    }
  }
}

function legacyProcessRunnerForced(env = process.env) {
  const devEnabled = envTrue(
    (env && (env.INFRING_DEV_ENABLE_LEGACY_PROCESS_RUNNER || env.PROTHEUS_DEV_ENABLE_LEGACY_PROCESS_RUNNER)) || ''
  );
  const forced = envTrue(
    (env && (env.INFRING_OPS_FORCE_LEGACY_PROCESS_RUNNER || env.PROTHEUS_OPS_FORCE_LEGACY_PROCESS_RUNNER)) || ''
  );
  if (!forced) return false;
  // legacy_process_runner_dev_only
  if (!devEnabled) return false;
  return !isProductionReleaseChannel(releaseChannel(env));
}

function resolveBinary(options = {}) {
  return resolveLegacyBinary(ROOT, options);
}

function writeAll(fd, text) {
  if (!text) return;
  const buffer = Buffer.from(text, 'utf8');
  let offset = 0;
  while (offset < buffer.length) {
    offset += fs.writeSync(fd, buffer, offset, buffer.length - offset);
  }
}

function invokeProtheusOpsViaBridge(args, options = {}) {
  if (!Array.isArray(args) || args.length === 0) return null;
  const domain = String(args[0] || '').trim();
  if (!domain || domain.startsWith('-')) return null;

  const passArgs = args.slice(1);
  const envOverrides = {};
  if (options.unknownDomainFallback === false) {
    envOverrides.INFRING_OPS_ALLOW_CARGO_FALLBACK = '0';
    envOverrides.PROTHEUS_OPS_ALLOW_CARGO_FALLBACK = '0';
  }
  const productionRelease = isProductionReleaseChannel(releaseChannel(process.env));
  if (options.allowProcessFallback === true && !productionRelease) {
    envOverrides.INFRING_OPS_ALLOW_PROCESS_FALLBACK = '1';
    envOverrides.PROTHEUS_OPS_ALLOW_PROCESS_FALLBACK = '1';
  } else if (options.allowProcessFallback === false) {
    envOverrides.INFRING_OPS_ALLOW_PROCESS_FALLBACK = '0';
    envOverrides.PROTHEUS_OPS_ALLOW_PROCESS_FALLBACK = '0';
  } else if (
    !Object.prototype.hasOwnProperty.call(process.env, 'INFRING_OPS_ALLOW_PROCESS_FALLBACK') &&
    !Object.prototype.hasOwnProperty.call(process.env, 'PROTHEUS_OPS_ALLOW_PROCESS_FALLBACK')
  ) {
    // Bridge-first default: keep process fallback disabled unless explicitly requested.
    envOverrides.INFRING_OPS_ALLOW_PROCESS_FALLBACK = '0';
    envOverrides.PROTHEUS_OPS_ALLOW_PROCESS_FALLBACK = '0';
  }

  const optionEnv =
    options && options.env && typeof options.env === 'object' ? options.env : {};
  const scopedEnv = { ...optionEnv, ...envOverrides };

  try {
    const bridgeOpts = {
      inheritStdio: true,
      preferLocalCore: true,
    };
    if (options.preferLocalCore === false) {
      bridgeOpts.preferLocalCore = false;
    }
    const bridge = createOpsLaneBridge(__dirname, 'run_protheus_ops', domain, bridgeOpts);
    return withScopedEnv(scopedEnv, () => bridge.run(passArgs));
  } catch {
    return null;
  }
}

function runProtheusOpsViaBridge(args, options = {}) {
  const out = invokeProtheusOpsViaBridge(args, options);
  if (!out) return null;
  if (out && out.stdout) writeAll(1, out.stdout);
  if (out && out.stderr) writeAll(2, out.stderr);
  if (out && out.payload && !out.stdout) {
    writeAll(1, `${JSON.stringify(out.payload)}\n`);
  }
  return Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1;
}

function runProtheusOpsLegacy(args, options = {}) {
  return runLegacyProcessRunner(ROOT, args, options);
}

function runProtheusOps(args, options = {}) {
  if (!legacyProcessRunnerForced(options && options.env ? options.env : process.env)) {
  const viaBridge = runProtheusOpsViaBridge(args, options);
    if (Number.isFinite(Number(viaBridge))) {
      return Number(viaBridge);
    }
  }
  return runProtheusOpsLegacy(args, options);
}

module.exports = {
  ROOT,
  resolveBinary,
  legacyProcessRunnerForced,
  invokeProtheusOpsViaBridge,
  runProtheusOps,
  runProtheusOpsViaBridge,
};

if (require.main === module) {
  const exitCode = runProtheusOps(process.argv.slice(2));
  process.exit(exitCode);
}
