#!/usr/bin/env node
'use strict';
export {};

/**
 * V4-RUST-001
 * Authoritative Rust microkernel cutover acceleration lane.
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
  clampNumber,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

type AnyObj = Record<string, any>;

const DEFAULT_POLICY_PATH = process.env.RUST_AUTHORITATIVE_MICROKERNEL_ACCELERATION_POLICY_PATH
  ? path.resolve(process.env.RUST_AUTHORITATIVE_MICROKERNEL_ACCELERATION_POLICY_PATH)
  : path.join(ROOT, 'config', 'rust_authoritative_microkernel_acceleration_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/ops/rust_authoritative_microkernel_acceleration.js run [--apply=1|0] [--strict=1|0] [--policy=<path>]');
  console.log('  node systems/ops/rust_authoritative_microkernel_acceleration.js report [--policy=<path>]');
  console.log('  node systems/ops/rust_authoritative_microkernel_acceleration.js status [--policy=<path>]');
}

function rel(filePath: string) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function parseJson(text: string) {
  const raw = String(text || '').trim();
  if (!raw) return null;
  try { return JSON.parse(raw); } catch {}
  const lines = raw.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try { return JSON.parse(lines[i]); } catch {}
  }
  return null;
}

function runNode(command: string[], envExtra: AnyObj = {}) {
  const started = Date.now();
  const run = spawnSync(command[0], command.slice(1), {
    cwd: ROOT,
    encoding: 'utf8',
    env: {
      ...process.env,
      ...envExtra
    },
    timeout: 180000
  });
  return {
    ok: Number(run.status || 0) === 0,
    status: Number.isFinite(run.status) ? Number(run.status) : 1,
    payload: parseJson(String(run.stdout || '')),
    stderr: cleanText(run.stderr || '', 320),
    duration_ms: Math.max(0, Date.now() - started)
  };
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    strict_default: true,
    targets: {
      rust_share_min_pct: 55,
      rust_share_max_pct: 65,
      enforce_target_during_cutover: false
    },
    scan: {
      include_extensions: ['.rs', '.ts', '.js'],
      ignore_roots: ['node_modules', 'dist', 'state', 'tmp', 'coverage']
    },
    commands: {
      rust_spine_parity: ['node', 'systems/ops/rust_spine_microkernel.js', 'parity', '--apply=1'],
      rust_spine_benchmark: ['node', 'systems/ops/rust_spine_microkernel.js', 'benchmark', '--apply=1', '--window=30'],
      rust_spine_cutover: ['node', 'systems/ops/rust_spine_microkernel.js', 'cutover', '--apply=1'],
      wasi2_gate: ['node', 'systems/ops/wasi2_execution_completeness_gate.js', 'run', '--apply=1', '--strict=1'],
      sandbox_coprocessor: ['node', 'systems/security/execution_sandbox_rust_wasm_coprocessor_lane.js', 'verify', '--owner=rust_accel', '--strict=1', '--apply=1', '--mock=1']
    },
    paths: {
      latest_path: 'state/ops/rust_authoritative_microkernel_acceleration/latest.json',
      receipts_path: 'state/ops/rust_authoritative_microkernel_acceleration/receipts.jsonl',
      language_report_path: 'state/ops/rust_authoritative_microkernel_acceleration/language_report.json'
    }
  };
}

function normalizeCommand(v: unknown) {
  if (!Array.isArray(v)) return [];
  return v.map((row) => cleanText(row, 260)).filter(Boolean);
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const targets = raw.targets && typeof raw.targets === 'object' ? raw.targets : {};
  const scan = raw.scan && typeof raw.scan === 'object' ? raw.scan : {};
  const commands = raw.commands && typeof raw.commands === 'object' ? raw.commands : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};

  return {
    version: cleanText(raw.version || base.version, 32),
    enabled: toBool(raw.enabled, true),
    strict_default: toBool(raw.strict_default, base.strict_default),
    targets: {
      rust_share_min_pct: clampNumber(targets.rust_share_min_pct, 0, 100, base.targets.rust_share_min_pct),
      rust_share_max_pct: clampNumber(targets.rust_share_max_pct, 0, 100, base.targets.rust_share_max_pct),
      enforce_target_during_cutover: toBool(targets.enforce_target_during_cutover, base.targets.enforce_target_during_cutover)
    },
    scan: {
      include_extensions: Array.isArray(scan.include_extensions)
        ? scan.include_extensions.map((v: unknown) => cleanText(v, 16)).filter(Boolean)
        : base.scan.include_extensions,
      ignore_roots: Array.isArray(scan.ignore_roots)
        ? scan.ignore_roots.map((v: unknown) => cleanText(v, 120)).filter(Boolean)
        : base.scan.ignore_roots
    },
    commands: {
      rust_spine_parity: normalizeCommand(commands.rust_spine_parity || base.commands.rust_spine_parity),
      rust_spine_benchmark: normalizeCommand(commands.rust_spine_benchmark || base.commands.rust_spine_benchmark),
      rust_spine_cutover: normalizeCommand(commands.rust_spine_cutover || base.commands.rust_spine_cutover),
      wasi2_gate: normalizeCommand(commands.wasi2_gate || base.commands.wasi2_gate),
      sandbox_coprocessor: normalizeCommand(commands.sandbox_coprocessor || base.commands.sandbox_coprocessor)
    },
    paths: {
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path),
      language_report_path: resolvePath(paths.language_report_path, base.paths.language_report_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function listFilesRecursive(rootPath: string, includeExtensions: string[], ignoreRoots: string[]) {
  const out: string[] = [];
  const ignore = new Set(ignoreRoots || []);
  if (!fs.existsSync(rootPath)) return out;
  const stack = [rootPath];
  while (stack.length) {
    const cur = stack.pop() as string;
    let entries: any[] = [];
    try {
      entries = fs.readdirSync(cur, { withFileTypes: true });
    } catch {
      entries = [];
    }
    entries.forEach((entry) => {
      const abs = path.join(cur, entry.name);
      if (entry.isDirectory()) {
        if (ignore.has(entry.name)) return;
        stack.push(abs);
      } else if (entry.isFile()) {
        if (includeExtensions.some((ext) => abs.endsWith(ext))) out.push(abs);
      }
    });
  }
  return out.sort((a, b) => a.localeCompare(b));
}

function computeLanguageReport(policy: AnyObj) {
  const files = listFilesRecursive(ROOT, policy.scan.include_extensions, policy.scan.ignore_roots);
  const buckets: AnyObj = { rs: { files: 0, bytes: 0 }, ts: { files: 0, bytes: 0 }, js: { files: 0, bytes: 0 } };

  files.forEach((filePath: string) => {
    let size = 0;
    try { size = Number(fs.statSync(filePath).size || 0); } catch {}
    if (filePath.endsWith('.rs')) {
      buckets.rs.files += 1;
      buckets.rs.bytes += size;
    } else if (filePath.endsWith('.ts')) {
      buckets.ts.files += 1;
      buckets.ts.bytes += size;
    } else if (filePath.endsWith('.js')) {
      buckets.js.files += 1;
      buckets.js.bytes += size;
    }
  });

  const totalBytes = buckets.rs.bytes + buckets.ts.bytes + buckets.js.bytes;
  const rustPct = totalBytes > 0 ? Number((100 * buckets.rs.bytes / totalBytes).toFixed(4)) : 0;
  const minTarget = Number.isFinite(Number(policy.targets?.rust_share_min_pct))
    ? Number(policy.targets.rust_share_min_pct)
    : 55;
  const maxTarget = Number.isFinite(Number(policy.targets?.rust_share_max_pct))
    ? Number(policy.targets.rust_share_max_pct)
    : 65;
  const bytesNeededForMin = totalBytes > 0
    ? Math.max(0, Math.ceil((minTarget / 100) * totalBytes) - buckets.rs.bytes)
    : 0;

  return {
    schema_id: 'rust_language_composition_report',
    schema_version: '1.0',
    ts: nowIso(),
    files_scanned: files.length,
    bytes: {
      rust: buckets.rs.bytes,
      ts: buckets.ts.bytes,
      js: buckets.js.bytes,
      total: totalBytes
    },
    files: {
      rust: buckets.rs.files,
      ts: buckets.ts.files,
      js: buckets.js.files,
      total: buckets.rs.files + buckets.ts.files + buckets.js.files
    },
    rust_share_pct: rustPct,
    target_range_pct: {
      min: minTarget,
      max: maxTarget
    },
    within_target_range: rustPct >= minTarget
      && rustPct <= maxTarget,
    bytes_needed_to_reach_min_target: bytesNeededForMin
  };
}

function runLane(args: AnyObj, policy: AnyObj) {
  const strict = args.strict != null ? toBool(args.strict, false) : policy.strict_default;
  const apply = toBool(args.apply, true);

  const execution = {
    rust_spine_parity: runNode(policy.commands.rust_spine_parity),
    rust_spine_benchmark: runNode(policy.commands.rust_spine_benchmark),
    rust_spine_cutover: runNode(policy.commands.rust_spine_cutover),
    wasi2_gate: runNode(policy.commands.wasi2_gate),
    sandbox_coprocessor: runNode(policy.commands.sandbox_coprocessor)
  };

  const report = computeLanguageReport(policy);
  writeJsonAtomic(policy.paths.language_report_path, report);

  const checks = {
    rust_spine_parity_ok: execution.rust_spine_parity.ok,
    rust_spine_benchmark_ok: execution.rust_spine_benchmark.ok,
    rust_spine_cutover_ok: execution.rust_spine_cutover.ok,
    wasi2_gate_ok: execution.wasi2_gate.ok,
    sandbox_coprocessor_ok: execution.sandbox_coprocessor.ok,
    rust_share_target_ok: report.within_target_range
  };

  const passRequired = checks.rust_spine_parity_ok
    && checks.rust_spine_benchmark_ok
    && checks.rust_spine_cutover_ok
    && checks.wasi2_gate_ok
    && checks.sandbox_coprocessor_ok
    && (policy.targets.enforce_target_during_cutover ? checks.rust_share_target_ok : true);

  const out = {
    ok: strict ? passRequired : true,
    pass_required_checks: passRequired,
    type: 'rust_authoritative_microkernel_acceleration',
    lane_id: 'V4-RUST-001',
    ts: nowIso(),
    strict,
    apply,
    checks,
    language_report_path: rel(policy.paths.language_report_path),
    language_report: report,
    executions: {
      rust_spine_parity: { ok: execution.rust_spine_parity.ok, status: execution.rust_spine_parity.status, duration_ms: execution.rust_spine_parity.duration_ms },
      rust_spine_benchmark: { ok: execution.rust_spine_benchmark.ok, status: execution.rust_spine_benchmark.status, duration_ms: execution.rust_spine_benchmark.duration_ms },
      rust_spine_cutover: { ok: execution.rust_spine_cutover.ok, status: execution.rust_spine_cutover.status, duration_ms: execution.rust_spine_cutover.duration_ms },
      wasi2_gate: { ok: execution.wasi2_gate.ok, status: execution.wasi2_gate.status, duration_ms: execution.wasi2_gate.duration_ms },
      sandbox_coprocessor: { ok: execution.sandbox_coprocessor.ok, status: execution.sandbox_coprocessor.status, duration_ms: execution.sandbox_coprocessor.duration_ms }
    },
    receipt_id: `rust_accel_${stableHash(JSON.stringify({ passRequired, report }), 12)}`
  };

  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  emit(out, out.ok ? 0 : 1);
}

function report(policy: AnyObj) {
  const lang = computeLanguageReport(policy);
  writeJsonAtomic(policy.paths.language_report_path, lang);
  emit({
    ok: true,
    type: 'rust_authoritative_microkernel_language_report',
    lane_id: 'V4-RUST-001',
    ...lang,
    language_report_path: rel(policy.paths.language_report_path)
  }, 0);
}

function status(policy: AnyObj) {
  emit({
    ok: true,
    type: 'rust_authoritative_microkernel_acceleration_status',
    lane_id: 'V4-RUST-001',
    latest: readJson(policy.paths.latest_path, null),
    language_report: readJson(policy.paths.language_report_path, null),
    latest_path: rel(policy.paths.latest_path),
    receipts_path: rel(policy.paths.receipts_path),
    language_report_path: rel(policy.paths.language_report_path)
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

  if (cmd === 'run') return runLane(args, policy);
  if (cmd === 'report') return report(policy);
  if (cmd === 'status') return status(policy);
  emit({ ok: false, error: 'unsupported_command', command: cmd }, 1);
}

main();
