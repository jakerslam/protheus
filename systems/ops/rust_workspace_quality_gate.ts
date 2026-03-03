#!/usr/bin/env node
'use strict';
export {};

/**
 * V4-RUST-003 foundation gate
 * Cargo workspace professionalization + CI quality signals.
 */

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
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  emit
} = require('../../lib/queued_backlog_runtime');

type AnyObj = Record<string, any>;

const DEFAULT_POLICY_PATH = process.env.RUST_WORKSPACE_QUALITY_GATE_POLICY_PATH
  ? path.resolve(process.env.RUST_WORKSPACE_QUALITY_GATE_POLICY_PATH)
  : path.join(ROOT, 'config', 'rust_workspace_quality_gate_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/ops/rust_workspace_quality_gate.js run [--strict=1|0] [--apply=1|0] [--policy=<path>]');
  console.log('  node systems/ops/rust_workspace_quality_gate.js status [--policy=<path>]');
}

function rel(filePath: string) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    strict_default: true,
    cargo_bin: 'cargo',
    checks: {
      enforce_workspace_manifest: true,
      enforce_toolchain_manifest: true,
      enforce_docs_generated: true,
      enforce_cargo_metadata: true,
      enforce_cargo_fmt: false,
      enforce_cargo_clippy: false,
      enforce_cargo_test: false
    },
    docs_required: [
      'docs/generated/TS_LANE_TYPE_REFERENCE.md',
      'docs/generated/RUST_LANE_TYPE_REFERENCE.md'
    ],
    commands: {
      metadata: ['metadata', '--format-version', '1', '--no-deps'],
      fmt: ['fmt', '--all', '--', '--check'],
      clippy: ['clippy', '--workspace', '--all-targets', '--all-features', '--', '-D', 'warnings'],
      test: ['test', '--workspace']
    },
    paths: {
      latest_path: 'state/ops/rust_workspace_quality_gate/latest.json',
      receipts_path: 'state/ops/rust_workspace_quality_gate/receipts.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const checks = raw.checks && typeof raw.checks === 'object' ? raw.checks : {};
  const commands = raw.commands && typeof raw.commands === 'object' ? raw.commands : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  const docsRequiredRaw = Array.isArray(raw.docs_required) ? raw.docs_required : base.docs_required;
  const docsRequired = docsRequiredRaw
    .map((v: unknown) => cleanText(v, 260))
    .filter(Boolean)
    .map((docPath: string) => (path.isAbsolute(docPath) ? docPath : path.join(ROOT, docPath)));

  return {
    version: cleanText(raw.version || base.version, 32),
    enabled: toBool(raw.enabled, true),
    strict_default: toBool(raw.strict_default, base.strict_default),
    cargo_bin: cleanText(raw.cargo_bin || base.cargo_bin, 80) || base.cargo_bin,
    checks: {
      enforce_workspace_manifest: toBool(checks.enforce_workspace_manifest, base.checks.enforce_workspace_manifest),
      enforce_toolchain_manifest: toBool(checks.enforce_toolchain_manifest, base.checks.enforce_toolchain_manifest),
      enforce_docs_generated: toBool(checks.enforce_docs_generated, base.checks.enforce_docs_generated),
      enforce_cargo_metadata: toBool(checks.enforce_cargo_metadata, base.checks.enforce_cargo_metadata),
      enforce_cargo_fmt: toBool(checks.enforce_cargo_fmt, base.checks.enforce_cargo_fmt),
      enforce_cargo_clippy: toBool(checks.enforce_cargo_clippy, base.checks.enforce_cargo_clippy),
      enforce_cargo_test: toBool(checks.enforce_cargo_test, base.checks.enforce_cargo_test)
    },
    docs_required: docsRequired,
    commands: {
      metadata: Array.isArray(commands.metadata) ? commands.metadata.map((v: unknown) => cleanText(v, 120)).filter(Boolean) : base.commands.metadata,
      fmt: Array.isArray(commands.fmt) ? commands.fmt.map((v: unknown) => cleanText(v, 120)).filter(Boolean) : base.commands.fmt,
      clippy: Array.isArray(commands.clippy) ? commands.clippy.map((v: unknown) => cleanText(v, 120)).filter(Boolean) : base.commands.clippy,
      test: Array.isArray(commands.test) ? commands.test.map((v: unknown) => cleanText(v, 120)).filter(Boolean) : base.commands.test
    },
    paths: {
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function runCargo(cargoBin: string, argv: string[]) {
  const run = spawnSync(cargoBin, argv, {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: 300000
  });
  return {
    ok: Number(run.status || 0) === 0,
    status: Number.isFinite(run.status) ? Number(run.status) : 1,
    stdout: String(run.stdout || ''),
    stderr: cleanText(run.stderr || '', 400)
  };
}

function runGate(args: AnyObj, policy: AnyObj) {
  const strict = args.strict != null ? toBool(args.strict, false) : policy.strict_default;
  const apply = toBool(args.apply, true);

  const workspaceManifestPath = path.join(ROOT, 'Cargo.toml');
  const toolchainPath = path.join(ROOT, 'rust-toolchain.toml');
  const workspaceManifestExists = fs.existsSync(workspaceManifestPath);
  const toolchainExists = fs.existsSync(toolchainPath);
  const missingDocs = policy.docs_required.filter((docPath: string) => !fs.existsSync(docPath));

  const metadata = runCargo(policy.cargo_bin, policy.commands.metadata);
  const fmt = policy.checks.enforce_cargo_fmt ? runCargo(policy.cargo_bin, policy.commands.fmt) : { ok: true, status: 0, stderr: null };
  const clippy = policy.checks.enforce_cargo_clippy ? runCargo(policy.cargo_bin, policy.commands.clippy) : { ok: true, status: 0, stderr: null };
  const test = policy.checks.enforce_cargo_test ? runCargo(policy.cargo_bin, policy.commands.test) : { ok: true, status: 0, stderr: null };

  const checks = {
    workspace_manifest_present: workspaceManifestExists,
    toolchain_manifest_present: toolchainExists,
    generated_docs_present: missingDocs.length === 0,
    cargo_metadata_ok: metadata.ok,
    cargo_fmt_ok: fmt.ok,
    cargo_clippy_ok: clippy.ok,
    cargo_test_ok: test.ok
  };

  const requiredChecks = {
    workspace_manifest_present: !policy.checks.enforce_workspace_manifest || checks.workspace_manifest_present,
    toolchain_manifest_present: !policy.checks.enforce_toolchain_manifest || checks.toolchain_manifest_present,
    generated_docs_present: !policy.checks.enforce_docs_generated || checks.generated_docs_present,
    cargo_metadata_ok: !policy.checks.enforce_cargo_metadata || checks.cargo_metadata_ok,
    cargo_fmt_ok: !policy.checks.enforce_cargo_fmt || checks.cargo_fmt_ok,
    cargo_clippy_ok: !policy.checks.enforce_cargo_clippy || checks.cargo_clippy_ok,
    cargo_test_ok: !policy.checks.enforce_cargo_test || checks.cargo_test_ok
  };

  const pass = Object.values(requiredChecks).every(Boolean);
  const out = {
    ok: strict ? pass : true,
    pass,
    type: 'rust_workspace_quality_gate',
    lane_id: 'V4-RUST-003',
    ts: nowIso(),
    strict,
    apply,
    checks,
    required_checks: requiredChecks,
    missing_docs: missingDocs,
    artifacts: {
      workspace_manifest: rel(workspaceManifestPath),
      toolchain_manifest: rel(toolchainPath),
      docs_required: policy.docs_required.map((docPath: string) => rel(docPath))
    },
    command_status: {
      metadata: { ok: metadata.ok, status: metadata.status, stderr: metadata.stderr },
      fmt: { ok: fmt.ok, status: fmt.status, stderr: fmt.stderr },
      clippy: { ok: clippy.ok, status: clippy.status, stderr: clippy.stderr },
      test: { ok: test.ok, status: test.status, stderr: test.stderr }
    }
  };

  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  emit(out, out.ok ? 0 : 1);
}

function status(policy: AnyObj) {
  emit({
    ok: true,
    type: 'rust_workspace_quality_gate_status',
    lane_id: 'V4-RUST-003',
    latest: readJson(policy.paths.latest_path, null),
    latest_path: rel(policy.paths.latest_path),
    receipts_path: rel(policy.paths.receipts_path)
  }, 0);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 80) || 'status';
  if (cmd === '--help' || cmd === '-h' || cmd === 'help') {
    usage();
    return;
  }

  const policy = loadPolicy(args.policy ? String(args.policy) : undefined);
  if (!policy.enabled) emit({ ok: false, error: 'policy_disabled' }, 1);
  if (cmd === 'run') return runGate(args, policy);
  if (cmd === 'status') return status(policy);
  emit({ ok: false, error: 'unsupported_command', command: cmd }, 1);
}

main();
