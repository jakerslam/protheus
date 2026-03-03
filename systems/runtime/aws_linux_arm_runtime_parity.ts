#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-232
 * AWS Linux/ARM runtime parity pack (AL2023 + Graviton + Bottlerocket).
 */

const fs = require('fs');
const path = require('path');
const os = require('os');
const {
  ROOT,
  nowIso,
  cleanText,
  toBool,
  parseArgs,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

type AnyObj = Record<string, any>;

const DEFAULT_POLICY_PATH = process.env.AWS_LINUX_ARM_RUNTIME_PARITY_POLICY_PATH
  ? path.resolve(process.env.AWS_LINUX_ARM_RUNTIME_PARITY_POLICY_PATH)
  : path.join(ROOT, 'config', 'aws_linux_arm_runtime_parity_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/runtime/aws_linux_arm_runtime_parity.js run [--host-os=linux --host-arch=arm64 --host-distro=al2023 --graviton=1 --neuron=1 --bottlerocket-profile=1 --strict=1] [--policy=<path>]');
  console.log('  node systems/runtime/aws_linux_arm_runtime_parity.js status [--policy=<path>]');
}

function rel(filePath: string) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    allowed_arches: ['arm64', 'aarch64'],
    allowed_distros: ['al2023', 'amazonlinux2023', 'bottlerocket'],
    required_capabilities: {
      graviton: true,
      neuron: true,
      bottlerocket_profile: true
    },
    fallback_runtime: 'cross_platform_runtime',
    rollback_command: 'node systems/runtime/aws_linux_arm_runtime_parity.js run --force-fallback=1',
    paths: {
      latest_path: 'state/runtime/aws_linux_arm_runtime_parity/latest.json',
      history_path: 'state/runtime/aws_linux_arm_runtime_parity/history.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const caps = raw.required_capabilities && typeof raw.required_capabilities === 'object' ? raw.required_capabilities : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 40) || base.version,
    enabled: raw.enabled !== false,
    allowed_arches: Array.isArray(raw.allowed_arches) && raw.allowed_arches.length
      ? raw.allowed_arches.map((v: unknown) => cleanText(v, 40).toLowerCase()).filter(Boolean)
      : base.allowed_arches,
    allowed_distros: Array.isArray(raw.allowed_distros) && raw.allowed_distros.length
      ? raw.allowed_distros.map((v: unknown) => cleanText(v, 80).toLowerCase()).filter(Boolean)
      : base.allowed_distros,
    required_capabilities: {
      graviton: caps.graviton !== false,
      neuron: caps.neuron !== false,
      bottlerocket_profile: caps.bottlerocket_profile !== false
    },
    fallback_runtime: cleanText(raw.fallback_runtime || base.fallback_runtime, 120) || base.fallback_runtime,
    rollback_command: cleanText(raw.rollback_command || base.rollback_command, 260) || base.rollback_command,
    paths: {
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function detectProbe(args: AnyObj) {
  const hostOs = cleanText(args['host-os'] || args.host_os || os.platform(), 40).toLowerCase();
  const hostArch = cleanText(args['host-arch'] || args.host_arch || os.arch(), 40).toLowerCase();
  const hostDistro = cleanText(args['host-distro'] || args.host_distro || process.env.AWS_HOST_DISTRO || '', 80).toLowerCase();

  return {
    host: {
      os: hostOs,
      arch: hostArch,
      distro: hostDistro,
      linux_family: hostOs === 'linux'
    },
    capabilities: {
      graviton: toBool(args.graviton ?? process.env.GRAVITON_AVAILABLE, false),
      neuron: toBool(args.neuron ?? process.env.NEURON_AVAILABLE, false),
      bottlerocket_profile: toBool(args['bottlerocket-profile'] ?? args.bottlerocket_profile ?? process.env.BOTTLEROCKET_PROFILE_AVAILABLE, false)
    }
  };
}

function evaluate(policy: AnyObj, probe: AnyObj, forceFallback: boolean) {
  const failures: string[] = [];

  if (!probe.host.linux_family) failures.push('host_not_linux');
  if (!policy.allowed_arches.includes(probe.host.arch)) failures.push('arch_not_supported');
  if (!policy.allowed_distros.includes(probe.host.distro)) failures.push('distro_not_supported');

  if (policy.required_capabilities.graviton === true && probe.capabilities.graviton !== true) failures.push('graviton_capability_missing');
  if (policy.required_capabilities.neuron === true && probe.capabilities.neuron !== true) failures.push('neuron_capability_missing');
  if (policy.required_capabilities.bottlerocket_profile === true && probe.capabilities.bottlerocket_profile !== true) failures.push('bottlerocket_profile_missing');

  if (forceFallback) failures.push('forced_fallback');

  const parityReady = failures.length === 0;
  return {
    parity_ready: parityReady,
    selected_runtime: parityReady ? 'aws_linux_arm_runtime' : policy.fallback_runtime,
    fallback_reason_codes: failures,
    rollback_safe_fallback: !parityReady,
    rollback_command: policy.rollback_command
  };
}

function runParity(args: AnyObj, policy: AnyObj) {
  if (policy.enabled !== true) {
    return {
      ok: true,
      type: 'aws_linux_arm_runtime_parity',
      ts: nowIso(),
      result: 'disabled_by_policy'
    };
  }

  const forceFallback = toBool(args['force-fallback'] ?? args.force_fallback, false);
  const probe = detectProbe(args);
  const evalOut = evaluate(policy, probe, forceFallback);

  return {
    ok: evalOut.parity_ready,
    type: 'aws_linux_arm_runtime_parity',
    lane_id: 'V3-RACE-232',
    ts: nowIso(),
    parity_receipt_id: `aws_arm_${stableHash(JSON.stringify({ probe, evalOut }), 14)}`,
    host: probe.host,
    capabilities: probe.capabilities,
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
    type: 'aws_linux_arm_runtime_parity_status',
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
