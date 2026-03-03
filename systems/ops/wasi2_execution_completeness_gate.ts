#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-220
 * WASI2 execution completeness gate (TS lanes -> WASM runtime path).
 */

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
  clampNumber,
  readJson,
  readJsonl,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  emit
} = require('../../lib/queued_backlog_runtime');

type AnyObj = Record<string, any>;

const DEFAULT_POLICY_PATH = process.env.WASI2_EXECUTION_COMPLETENESS_GATE_POLICY_PATH
  ? path.resolve(process.env.WASI2_EXECUTION_COMPLETENESS_GATE_POLICY_PATH)
  : path.join(ROOT, 'config', 'wasi2_execution_completeness_gate_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/ops/wasi2_execution_completeness_gate.js run [--apply=1|0] [--strict=1|0] [--window=30] [--policy=<path>]');
  console.log('  node systems/ops/wasi2_execution_completeness_gate.js status [--policy=<path>]');
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

function runProbe(lane: string, engine: 'js' | 'wasi2', owner: string) {
  const cmd = [
    'node',
    'systems/ops/wasi2_lane_adapter.js',
    'probe',
    `--lane=${lane}`,
    `--engine=${engine}`,
    `--owner=${owner}`
  ];
  const started = Date.now();
  const run = spawnSync(cmd[0], cmd.slice(1), {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: 120000
  });
  return {
    ok: Number(run.status || 0) === 0,
    code: Number.isFinite(run.status) ? Number(run.status) : 1,
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
    thresholds: {
      min_parity_pass_rate: 1,
      max_p95_latency_delta_ms: 250,
      min_safety_pass_rate: 1
    },
    contract_fields: ['type', 'lane', 'contract_version', 'health'],
    target_lanes: ['guard', 'spawn_broker', 'model_router', 'origin_lock', 'fractal_orchestrator'],
    owner_id: 'wasi2_gate',
    paths: {
      state_path: 'state/ops/wasi2_execution_completeness_gate/state.json',
      latest_path: 'state/ops/wasi2_execution_completeness_gate/latest.json',
      receipts_path: 'state/ops/wasi2_execution_completeness_gate/receipts.jsonl',
      history_path: 'state/ops/wasi2_execution_completeness_gate/history.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const thresholds = raw.thresholds && typeof raw.thresholds === 'object' ? raw.thresholds : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 32),
    enabled: toBool(raw.enabled, true),
    strict_default: toBool(raw.strict_default, base.strict_default),
    thresholds: {
      min_parity_pass_rate: clampNumber(thresholds.min_parity_pass_rate, 0, 1, base.thresholds.min_parity_pass_rate),
      max_p95_latency_delta_ms: clampNumber(thresholds.max_p95_latency_delta_ms, 0, 120000, base.thresholds.max_p95_latency_delta_ms),
      min_safety_pass_rate: clampNumber(thresholds.min_safety_pass_rate, 0, 1, base.thresholds.min_safety_pass_rate)
    },
    contract_fields: Array.isArray(raw.contract_fields)
      ? raw.contract_fields.map((v: unknown) => normalizeToken(v, 80)).filter(Boolean)
      : base.contract_fields,
    target_lanes: Array.isArray(raw.target_lanes)
      ? raw.target_lanes.map((v: unknown) => normalizeToken(v, 120)).filter(Boolean)
      : base.target_lanes,
    owner_id: normalizeToken(raw.owner_id || base.owner_id, 120) || base.owner_id,
    paths: {
      state_path: resolvePath(paths.state_path, base.paths.state_path),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function contractMatch(fields: string[], left: AnyObj, right: AnyObj) {
  for (const key of fields) {
    const lk = left ? left[key] : undefined;
    const rk = right ? right[key] : undefined;
    if (JSON.stringify(lk) !== JSON.stringify(rk)) return false;
  }
  return true;
}

function percentile(values: number[], p: number) {
  if (!Array.isArray(values) || values.length < 1) return 0;
  const sorted = values.map((n) => Number(n || 0)).sort((a, b) => a - b);
  const idx = Math.min(sorted.length - 1, Math.max(0, Math.floor((p / 100) * sorted.length)));
  return sorted[idx];
}

function runGate(args: AnyObj, policy: AnyObj) {
  const strict = args.strict != null ? toBool(args.strict, false) : policy.strict_default;
  const apply = toBool(args.apply, true);
  const rows: AnyObj[] = [];
  const deltas: number[] = [];

  for (const lane of policy.target_lanes) {
    const js = runProbe(lane, 'js', policy.owner_id);
    const wasi2 = runProbe(lane, 'wasi2', policy.owner_id);
    const jsNorm = js.payload && js.payload.normalized && typeof js.payload.normalized === 'object' ? js.payload.normalized : null;
    const wasi2Norm = wasi2.payload && wasi2.payload.normalized && typeof wasi2.payload.normalized === 'object' ? wasi2.payload.normalized : null;
    const parity = js.ok && wasi2.ok && contractMatch(policy.contract_fields, jsNorm, wasi2Norm);
    const safety = wasi2.ok && (!wasi2.payload || !wasi2.payload.stderr);
    const latencyDelta = Math.abs(Number(js.duration_ms || 0) - Number(wasi2.duration_ms || 0));
    deltas.push(latencyDelta);
    rows.push({
      lane,
      js_ok: js.ok,
      wasi2_ok: wasi2.ok,
      parity_pass: parity,
      safety_pass: safety,
      js_duration_ms: js.duration_ms,
      wasi2_duration_ms: wasi2.duration_ms,
      latency_delta_ms: latencyDelta,
      js_error: js.ok ? null : js.stderr,
      wasi2_error: wasi2.ok ? null : wasi2.stderr
    });
  }

  const total = rows.length;
  const parityPassCount = rows.filter((row) => row.parity_pass === true).length;
  const safetyPassCount = rows.filter((row) => row.safety_pass === true).length;
  const parityPassRate = total > 0 ? Number((parityPassCount / total).toFixed(6)) : 0;
  const safetyPassRate = total > 0 ? Number((safetyPassCount / total).toFixed(6)) : 0;
  const p95Delta = Number(percentile(deltas, 95).toFixed(3));

  const checks = {
    parity_pass_rate_ok: parityPassRate >= Number(policy.thresholds.min_parity_pass_rate || 1),
    safety_pass_rate_ok: safetyPassRate >= Number(policy.thresholds.min_safety_pass_rate || 1),
    latency_p95_delta_ok: p95Delta <= Number(policy.thresholds.max_p95_latency_delta_ms || 250)
  };
  const pass = Object.values(checks).every(Boolean);

  const out = {
    ok: strict ? pass : true,
    pass,
    type: 'wasi2_execution_completeness_gate',
    lane_id: 'V3-RACE-220',
    ts: nowIso(),
    strict,
    apply,
    checks,
    target_lane_count: total,
    parity_pass_count: parityPassCount,
    safety_pass_count: safetyPassCount,
    parity_pass_rate: parityPassRate,
    safety_pass_rate: safetyPassRate,
    p95_latency_delta_ms: p95Delta,
    rows
  };

  if (apply) {
    writeJsonAtomic(policy.paths.state_path, {
      schema_id: 'wasi2_execution_completeness_gate_state',
      schema_version: '1.0',
      updated_at: out.ts,
      last_run: {
        ts: out.ts,
        pass,
        checks,
        parity_pass_rate: parityPassRate,
        safety_pass_rate: safetyPassRate,
        p95_latency_delta_ms: p95Delta
      }
    });
    appendJsonl(policy.paths.history_path, out);
  }

  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  emit(out, out.ok ? 0 : 1);
}

function status(policy: AnyObj) {
  const history = readJsonl(policy.paths.history_path);
  emit({
    ok: true,
    type: 'wasi2_execution_completeness_gate_status',
    lane_id: 'V3-RACE-220',
    latest: readJson(policy.paths.latest_path, null),
    state: readJson(policy.paths.state_path, null),
    run_count: history.length,
    latest_path: path.relative(ROOT, policy.paths.latest_path).replace(/\\/g, '/'),
    receipts_path: path.relative(ROOT, policy.paths.receipts_path).replace(/\\/g, '/'),
    history_path: path.relative(ROOT, policy.paths.history_path).replace(/\\/g, '/')
  }, 0);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 40) || 'status';
  if (cmd === '--help' || cmd === '-h' || cmd === 'help') {
    usage();
    return;
  }

  const policy = loadPolicy(args.policy ? String(args.policy) : undefined);
  if (!policy.enabled) emit({ ok: false, error: 'policy_disabled' }, 1);
  if (cmd === 'run' || cmd === 'verify') return runGate(args, policy);
  if (cmd === 'status') return status(policy);
  emit({ ok: false, error: 'unsupported_command', command: cmd }, 1);
}

main();
