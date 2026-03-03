#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  toBool,
  clampInt,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  emit
} = require('../../../lib/queued_backlog_runtime');

type AnyObj = Record<string, any>;

const DEFAULT_POLICY_PATH = process.env.PROTHEUS_MOBILE_ADAPTER_POLICY_PATH
  ? path.resolve(process.env.PROTHEUS_MOBILE_ADAPTER_POLICY_PATH)
  : path.join(ROOT, 'config', 'protheus_mobile_adapter_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/hybrid/mobile/protheus_mobile_adapter.js status [--policy=<path>]');
  console.log('  node systems/hybrid/mobile/protheus_mobile_adapter.js manifest [--apply=1|0] [--policy=<path>]');
  console.log('  node systems/hybrid/mobile/protheus_mobile_adapter.js build [--apply=1|0] [--strict=1|0] [--policy=<path>]');
}

function rel(absPath: string) {
  return path.relative(ROOT, absPath).replace(/\\/g, '/');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    strict_default: false,
    targets: {
      background_battery_max_pct_24h: 5,
      max_build_ms: 900000
    },
    manifests: {
      output_path: 'state/hybrid/mobile_adapter/mobile_manifest.json'
    },
    build: {
      rust_release_cmd: ['cargo', 'build', '--manifest-path', 'systems/hybrid/rust/Cargo.toml', '--release', '--quiet'],
      wasm_target: 'wasm32-unknown-unknown',
      wasm_build_cmd: ['cargo', 'build', '--manifest-path', 'systems/hybrid/rust/Cargo.toml', '--target', 'wasm32-unknown-unknown', '--release', '--quiet']
    },
    paths: {
      latest_path: 'state/hybrid/mobile_adapter/latest.json',
      receipts_path: 'state/hybrid/mobile_adapter/receipts.jsonl',
      history_path: 'state/hybrid/mobile_adapter/history.jsonl',
      state_path: 'state/hybrid/mobile_adapter/state.json'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const base = defaultPolicy();
  const raw = readJson(policyPath, {});
  const targets = raw.targets && typeof raw.targets === 'object' ? raw.targets : {};
  const manifests = raw.manifests && typeof raw.manifests === 'object' ? raw.manifests : {};
  const build = raw.build && typeof raw.build === 'object' ? raw.build : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};

  function arr(input: unknown, fallback: string[]) {
    if (!Array.isArray(input)) return fallback;
    const out = input.map((v) => cleanText(v, 260)).filter(Boolean);
    return out.length ? out : fallback;
  }

  return {
    version: cleanText(raw.version || base.version, 24) || '1.0',
    enabled: toBool(raw.enabled, true),
    strict_default: toBool(raw.strict_default, base.strict_default),
    targets: {
      background_battery_max_pct_24h: Number.isFinite(Number(targets.background_battery_max_pct_24h))
        ? Number(targets.background_battery_max_pct_24h)
        : Number(base.targets.background_battery_max_pct_24h),
      max_build_ms: clampInt(targets.max_build_ms, 1000, 3600000, base.targets.max_build_ms)
    },
    manifests: {
      output_path: resolvePath(manifests.output_path, base.manifests.output_path)
    },
    build: {
      rust_release_cmd: arr(build.rust_release_cmd, base.build.rust_release_cmd),
      wasm_target: cleanText(build.wasm_target || base.build.wasm_target, 120) || base.build.wasm_target,
      wasm_build_cmd: arr(build.wasm_build_cmd, base.build.wasm_build_cmd)
    },
    paths: {
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path),
      state_path: resolvePath(paths.state_path, base.paths.state_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function commandExists(commandName: string) {
  const shell = process.env.SHELL || '/bin/zsh';
  const out = spawnSync(shell, ['-lc', `command -v ${commandName}`], {
    cwd: ROOT,
    encoding: 'utf8'
  });
  const status = Number.isFinite(Number(out.status)) ? Number(out.status) : 1;
  return status === 0;
}

function quoteArg(v: string) {
  return `'${String(v).replace(/'/g, `'\\''`)}'`;
}

function runStep(step: string, command: string[], timeoutMs: number) {
  const started = Date.now();
  const shell = process.env.SHELL || '/bin/zsh';
  const commandString = command.map((token) => quoteArg(token)).join(' ');
  const out = spawnSync(shell, ['-lc', commandString], {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: Math.max(1000, timeoutMs)
  });
  const status = Number.isFinite(Number(out.status)) ? Number(out.status) : 1;
  return {
    step,
    command,
    ok: status === 0,
    status,
    duration_ms: Math.max(0, Date.now() - started),
    stderr: cleanText(out.stderr || '', 400),
    stdout_tail: cleanText(String(out.stdout || '').slice(-220), 220)
  };
}

function listInstalledRustTargets() {
  const out = spawnSync('rustup', ['target', 'list', '--installed'], {
    cwd: ROOT,
    encoding: 'utf8'
  });
  const status = Number.isFinite(Number(out.status)) ? Number(out.status) : 1;
  if (status !== 0) return [];
  return String(out.stdout || '')
    .split('\n')
    .map((line) => cleanText(line, 120))
    .filter(Boolean);
}

function buildManifest() {
  return {
    schema_id: 'protheus_mobile_adapter_manifest',
    schema_version: '1.0',
    ts: nowIso(),
    adapter: 'protheus-mobile',
    transports: {
      wasm: {
        entrypoint: 'systems/hybrid/rust',
        target: 'wasm32-unknown-unknown'
      },
      tauri: {
        ios: {
          service: 'ProtheusBackgroundLane',
          mode: 'background_fetch',
          min_interval_minutes: 15
        },
        android: {
          service: 'ProtheusForegroundService',
          mode: 'work_manager',
          min_interval_minutes: 15
        }
      }
    },
    binary_interfaces: [
      'memory-hotpath',
      'execution-replay',
      'crdt-merge',
      'security-vault',
      'red-chaos',
      'telemetry-emit'
    ],
    runtime_contract: {
      deterministic_receipts: true,
      fail_closed_on_missing_binding: true,
      fallback_lane: 'ts_adapters'
    }
  };
}

function estimateBatteryImpactPct24h(matrix: AnyObj[]) {
  const totalMs = matrix.reduce((acc, row) => acc + Number(row.duration_ms || 0), 0);
  const failedSteps = matrix.filter((row) => row.ok !== true).length;
  const baseline = (totalMs / 900000) * 1.6;
  const penalty = failedSteps * 0.3;
  return Number((baseline + penalty).toFixed(3));
}

function persistArtifacts(policy: AnyObj, receipt: AnyObj, apply: boolean) {
  if (!apply) return;
  fs.mkdirSync(path.dirname(policy.paths.latest_path), { recursive: true });
  fs.mkdirSync(path.dirname(policy.paths.receipts_path), { recursive: true });
  fs.mkdirSync(path.dirname(policy.paths.history_path), { recursive: true });
  fs.mkdirSync(path.dirname(policy.paths.state_path), { recursive: true });
  fs.mkdirSync(path.dirname(policy.manifests.output_path), { recursive: true });
  writeJsonAtomic(policy.paths.latest_path, receipt);
  writeJsonAtomic(policy.paths.state_path, receipt);
  appendJsonl(policy.paths.receipts_path, receipt);
  appendJsonl(policy.paths.history_path, receipt);
}

function cmdManifest(policy: AnyObj, apply: boolean) {
  const manifest = buildManifest();
  if (apply) {
    fs.mkdirSync(path.dirname(policy.manifests.output_path), { recursive: true });
    writeJsonAtomic(policy.manifests.output_path, manifest);
  }
  return {
    ok: true,
    type: 'protheus_mobile_adapter',
    action: 'manifest',
    ts: nowIso(),
    policy_path: rel(policy.policy_path),
    manifest_path: rel(policy.manifests.output_path),
    manifest
  };
}

function cmdBuild(policy: AnyObj, apply: boolean, strict: boolean) {
  const manifestOut = cmdManifest(policy, apply);
  const targetList = listInstalledRustTargets();
  const wasmInstalled = targetList.includes(policy.build.wasm_target);
  const tauriInstalled = commandExists('tauri');
  const cargoInstalled = commandExists('cargo');

  const matrix: AnyObj[] = [];
  if (cargoInstalled) {
    matrix.push(runStep('rust_release', policy.build.rust_release_cmd, policy.targets.max_build_ms));
  } else {
    matrix.push({
      step: 'rust_release',
      command: policy.build.rust_release_cmd,
      ok: false,
      skipped: true,
      reason: 'cargo_missing',
      duration_ms: 0
    });
  }

  if (cargoInstalled && wasmInstalled) {
    matrix.push(runStep('wasm_release', policy.build.wasm_build_cmd, policy.targets.max_build_ms));
  } else {
    matrix.push({
      step: 'wasm_release',
      command: policy.build.wasm_build_cmd,
      ok: false,
      skipped: true,
      reason: wasmInstalled ? 'cargo_missing' : 'wasm_target_missing',
      duration_ms: 0
    });
  }

  matrix.push({
    step: 'tauri_toolchain',
    command: ['tauri', '--version'],
    ok: tauriInstalled,
    skipped: !tauriInstalled,
    reason: tauriInstalled ? '' : 'tauri_missing',
    duration_ms: 0
  });

  const batteryImpact = estimateBatteryImpactPct24h(matrix);
  const checks = {
    manifest_written: manifestOut.ok === true,
    cargo_available: cargoInstalled,
    wasm_target_installed: wasmInstalled,
    rust_release_ok: matrix.some((row) => row.step === 'rust_release' && row.ok === true),
    wasm_release_ok: matrix.some((row) => row.step === 'wasm_release' && row.ok === true),
    tauri_available: tauriInstalled,
    battery_within_target: batteryImpact <= Number(policy.targets.background_battery_max_pct_24h)
  };

  const runOk = checks.manifest_written
    && checks.rust_release_ok
    && checks.wasm_release_ok
    && checks.battery_within_target;

  const receipt = {
    schema_id: 'protheus_mobile_adapter_receipt',
    schema_version: '1.0',
    artifact_type: 'receipt',
    ok: strict ? runOk : true,
    type: 'protheus_mobile_adapter',
    action: 'build',
    ts: nowIso(),
    strict,
    policy_path: rel(policy.policy_path),
    manifest_path: rel(policy.manifests.output_path),
    targets_installed: targetList,
    checks,
    build_matrix: matrix,
    summary: {
      background_battery_pct_24h: batteryImpact,
      background_battery_target_pct_24h: policy.targets.background_battery_max_pct_24h
    },
    status: runOk ? 'ready' : 'degraded'
  };

  persistArtifacts(policy, receipt, apply);
  return receipt;
}

function cmdStatus(policy: AnyObj) {
  return {
    ok: true,
    type: 'protheus_mobile_adapter',
    action: 'status',
    ts: nowIso(),
    policy_path: rel(policy.policy_path),
    state: readJson(policy.paths.state_path, null),
    manifest: readJson(policy.manifests.output_path, null)
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 80) || 'status';
  if (cmd === '--help' || cmd === '-h' || cmd === 'help') {
    usage();
    process.exit(0);
  }
  const policyPath = args.policy
    ? (path.isAbsolute(String(args.policy)) ? String(args.policy) : path.join(ROOT, String(args.policy)))
    : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  if (!policy.enabled) emit({ ok: false, error: 'mobile_adapter_disabled' }, 1);

  const apply = toBool(args.apply, true);
  const strict = args.strict != null ? toBool(args.strict, policy.strict_default) : policy.strict_default;

  if (cmd === 'status') emit(cmdStatus(policy), 0);
  if (cmd === 'manifest') emit(cmdManifest(policy, apply), 0);
  if (cmd === 'build') {
    const out = cmdBuild(policy, apply, strict);
    emit(out, out.ok ? 0 : 1);
  }

  usage();
  process.exit(1);
}

main();
