#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-262
 * Google ecosystem runtime parity pack (Android/ChromeOS/Fuchsia).
 */

const fs = require('fs');
const path = require('path');
const {
  ROOT,
  nowIso,
  cleanText,
  toBool,
  clampInt,
  parseArgs,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

type AnyObj = Record<string, any>;

const DEFAULT_POLICY_PATH = process.env.GOOGLE_ECOSYSTEM_RUNTIME_PARITY_POLICY_PATH
  ? path.resolve(process.env.GOOGLE_ECOSYSTEM_RUNTIME_PARITY_POLICY_PATH)
  : path.join(ROOT, 'config', 'google_ecosystem_runtime_parity_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/runtime/google_ecosystem_runtime_parity.js run [--android=1 --chromeos=1 --fuchsia=1 --android-privileged=1 --chromeos-privileged=1 --strict=1] [--policy=<path>]');
  console.log('  node systems/runtime/google_ecosystem_runtime_parity.js status [--policy=<path>]');
}

function rel(filePath: string) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    required_surfaces: ['android', 'chromeos', 'fuchsia'],
    privileged_service_requirements: {
      android: true,
      chromeos: true,
      fuchsia: false
    },
    min_android_api_level: 16,
    fallback_runtime: 'baseline_mobile_runtime',
    rollback_command: 'node systems/runtime/google_ecosystem_runtime_parity.js run --force-fallback=1',
    paths: {
      latest_path: 'state/runtime/google_ecosystem_runtime_parity/latest.json',
      history_path: 'state/runtime/google_ecosystem_runtime_parity/history.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const reqPriv = raw.privileged_service_requirements && typeof raw.privileged_service_requirements === 'object'
    ? raw.privileged_service_requirements
    : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};

  return {
    version: cleanText(raw.version || base.version, 40) || base.version,
    enabled: raw.enabled !== false,
    required_surfaces: Array.isArray(raw.required_surfaces) && raw.required_surfaces.length
      ? raw.required_surfaces.map((v: unknown) => cleanText(v, 80).toLowerCase()).filter(Boolean)
      : base.required_surfaces,
    privileged_service_requirements: {
      android: reqPriv.android !== false,
      chromeos: reqPriv.chromeos !== false,
      fuchsia: reqPriv.fuchsia === true
    },
    min_android_api_level: clampInt(raw.min_android_api_level, 1, 100, base.min_android_api_level),
    fallback_runtime: cleanText(raw.fallback_runtime || base.fallback_runtime, 120) || base.fallback_runtime,
    rollback_command: cleanText(raw.rollback_command || base.rollback_command, 260) || base.rollback_command,
    paths: {
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function detectSurfaces(args: AnyObj) {
  const androidApi = clampInt(args['android-api'] ?? args.android_api, 0, 100, 0);
  return {
    surfaces: {
      android: toBool(args.android ?? process.env.ANDROID_SURFACE_AVAILABLE, false),
      chromeos: toBool(args.chromeos ?? process.env.CHROMEOS_SURFACE_AVAILABLE, false),
      fuchsia: toBool(args.fuchsia ?? process.env.FUCHSIA_SURFACE_AVAILABLE, false)
    },
    privileged: {
      android: toBool(args['android-privileged'] ?? args.android_privileged ?? process.env.ANDROID_PRIVILEGED_AVAILABLE, false),
      chromeos: toBool(args['chromeos-privileged'] ?? args.chromeos_privileged ?? process.env.CHROMEOS_PRIVILEGED_AVAILABLE, false),
      fuchsia: toBool(args['fuchsia-privileged'] ?? args.fuchsia_privileged ?? process.env.FUCHSIA_PRIVILEGED_AVAILABLE, false)
    },
    android_api_level: androidApi
  };
}

function evaluate(policy: AnyObj, probe: AnyObj, forceFallback: boolean) {
  const failures: string[] = [];
  const surfaceMatrix: AnyObj[] = [];

  for (const surface of policy.required_surfaces) {
    const available = probe.surfaces[surface] === true;
    const privilegedRequired = policy.privileged_service_requirements[surface] === true;
    const privilegedReady = privilegedRequired ? probe.privileged[surface] === true : true;

    if (!available) failures.push(`${surface}_surface_missing`);
    if (!privilegedReady) failures.push(`${surface}_privileged_service_missing`);

    surfaceMatrix.push({
      surface,
      available,
      privileged_required: privilegedRequired,
      privileged_ready: privilegedReady,
      parity_ready: available && privilegedReady
    });
  }

  if (probe.surfaces.android && Number(probe.android_api_level || 0) < Number(policy.min_android_api_level || 0)) {
    failures.push('android_api_level_too_low');
  }

  if (forceFallback) failures.push('forced_fallback');

  const parityReady = failures.length === 0;
  return {
    parity_ready: parityReady,
    selected_runtime: parityReady ? 'google_ecosystem_runtime' : policy.fallback_runtime,
    fallback_reason_codes: failures,
    surface_matrix: surfaceMatrix,
    rollback_safe_fallback: !parityReady,
    rollback_command: policy.rollback_command
  };
}

function runParity(args: AnyObj, policy: AnyObj) {
  if (policy.enabled !== true) {
    return {
      ok: true,
      type: 'google_ecosystem_runtime_parity',
      ts: nowIso(),
      result: 'disabled_by_policy'
    };
  }

  const forceFallback = toBool(args['force-fallback'] ?? args.force_fallback, false);
  const probe = detectSurfaces(args);
  const evalOut = evaluate(policy, probe, forceFallback);

  return {
    ok: evalOut.parity_ready,
    type: 'google_ecosystem_runtime_parity',
    lane_id: 'V3-RACE-262',
    ts: nowIso(),
    parity_receipt_id: `google_parity_${stableHash(JSON.stringify({ probe, evalOut }), 14)}`,
    required_surfaces: policy.required_surfaces,
    android_api_level: probe.android_api_level,
    surface_matrix: evalOut.surface_matrix,
    parity_ready: evalOut.parity_ready,
    selected_runtime: evalOut.selected_runtime,
    fallback_reason_codes: evalOut.fallback_reason_codes,
    rollback_safe_fallback: evalOut.rollback_safe_fallback,
    rollback_command: evalOut.rollback_command
  };
}

function cmdRun(args: AnyObj) {
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  const out = runParity(args, policy);

  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.history_path, {
    ts: out.ts,
    type: out.type,
    ok: out.ok,
    selected_runtime: out.selected_runtime,
    fallback_reason_codes: out.fallback_reason_codes
  });

  emit({
    ...out,
    policy_path: rel(policy.policy_path),
    latest_path: rel(policy.paths.latest_path)
  }, out.ok || !toBool(args.strict, false) ? 0 : 1);
}

function cmdStatus(args: AnyObj) {
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  emit({
    ok: true,
    type: 'google_ecosystem_runtime_parity_status',
    ts: nowIso(),
    latest: readJson(policy.paths.latest_path, null),
    policy_path: rel(policy.policy_path),
    latest_path: rel(policy.paths.latest_path)
  }, 0);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'status', 80).toLowerCase();
  if (args.help || ['help', '--help', '-h'].includes(cmd)) {
    usage();
    process.exit(0);
  }

  if (cmd === 'run') return cmdRun(args);
  if (cmd === 'status') return cmdStatus(args);

  usage();
  emit({ ok: false, error: `unknown_command:${cmd}` }, 2);
}

main();
