#!/usr/bin/env node
'use strict';
export {};

/**
 * BL-019
 * Optional habit cell-pool executor with bounded dynamic concurrency.
 *
 * Usage:
 *   node systems/habits/habit_cell_pool_executor.js plan --queue-json='[{"id":"habit_a"}]'
 *   node systems/habits/habit_cell_pool_executor.js execute --queue-json='[{"id":"habit_a"}]' [--apply=1|0] [--strict=1|0]
 *   node systems/habits/habit_cell_pool_executor.js status
 */

const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');

type AnyObj = Record<string, any>;

const ROOT = process.env.HABIT_CELL_POOL_EXECUTOR_ROOT
  ? path.resolve(process.env.HABIT_CELL_POOL_EXECUTOR_ROOT)
  : path.resolve(__dirname, '..', '..');

const DEFAULT_POLICY_PATH = process.env.HABIT_CELL_POOL_EXECUTOR_POLICY_PATH
  ? path.resolve(process.env.HABIT_CELL_POOL_EXECUTOR_POLICY_PATH)
  : path.join(ROOT, 'config', 'habit_cell_pool_executor_policy.json');

function nowIso() { return new Date().toISOString(); }
function cleanText(v: unknown, maxLen = 360) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}
function parseArgs(argv: string[]) {
  const out: AnyObj = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const tok = String(argv[i] || '');
    if (!tok.startsWith('--')) { out._.push(tok); continue; }
    const eq = tok.indexOf('=');
    if (eq >= 0) { out[tok.slice(2, eq)] = tok.slice(eq + 1); continue; }
    const key = tok.slice(2);
    const next = argv[i + 1];
    if (next != null && !String(next).startsWith('--')) { out[key] = String(next); i += 1; continue; }
    out[key] = true;
  }
  return out;
}
function toBool(v: unknown, fallback = false) {
  if (v == null) return fallback;
  const raw = String(v).trim().toLowerCase();
  if (['1', 'true', 'yes', 'on'].includes(raw)) return true;
  if (['0', 'false', 'no', 'off'].includes(raw)) return false;
  return fallback;
}
function clampInt(v: unknown, lo: number, hi: number, fallback: number) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  const x = Math.trunc(n);
  if (x < lo) return lo;
  if (x > hi) return hi;
  return x;
}
function ensureDir(dirPath: string) { fs.mkdirSync(dirPath, { recursive: true }); }
function readJson(filePath: string, fallback: any = null) {
  try { if (!fs.existsSync(filePath)) return fallback; const parsed = JSON.parse(fs.readFileSync(filePath, 'utf8')); return parsed == null ? fallback : parsed; } catch { return fallback; }
}
function writeJsonAtomic(filePath: string, value: AnyObj) {
  ensureDir(path.dirname(filePath));
  const tmp = `${filePath}.tmp-${Date.now()}-${process.pid}`;
  fs.writeFileSync(tmp, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
  fs.renameSync(tmp, filePath);
}
function appendJsonl(filePath: string, row: AnyObj) {
  ensureDir(path.dirname(filePath));
  fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`, 'utf8');
}
function resolvePath(raw: unknown, fallbackRel: string) {
  const txt = cleanText(raw, 520);
  if (!txt) return path.join(ROOT, fallbackRel);
  return path.isAbsolute(txt) ? txt : path.join(ROOT, txt);
}
function rel(filePath: string) { return path.relative(ROOT, filePath).replace(/\\/g, '/'); }
function parseJsonArg(raw: unknown, fallback: any = null) {
  const txt = cleanText(raw, 120000);
  if (!txt) return fallback;
  try { return JSON.parse(txt); } catch { return fallback; }
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: false,
    bounds: {
      min_workers: 1,
      max_workers: 6
    },
    hysteresis: {
      scale_up_queue_threshold: 4,
      scale_down_queue_threshold: 1,
      cooldown_sec: 180
    },
    safety: {
      allowed_risks: ['low', 'medium'],
      deny_habit_ids: [],
      require_explicit_allow: false
    },
    execution: {
      runner_path: 'habits/scripts/run_habit.js',
      apply_default: false,
      payload_json_default: '{}'
    },
    outputs: {
      state_path: 'state/habits/habit_cell_pool_executor/state.json',
      latest_path: 'state/habits/habit_cell_pool_executor/latest.json',
      history_path: 'state/habits/habit_cell_pool_executor/history.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const base = defaultPolicy();
  const raw = readJson(policyPath, {});
  const bounds = raw.bounds && typeof raw.bounds === 'object' ? raw.bounds : {};
  const hysteresis = raw.hysteresis && typeof raw.hysteresis === 'object' ? raw.hysteresis : {};
  const safety = raw.safety && typeof raw.safety === 'object' ? raw.safety : {};
  const execution = raw.execution && typeof raw.execution === 'object' ? raw.execution : {};
  const outputs = raw.outputs && typeof raw.outputs === 'object' ? raw.outputs : {};

  const minWorkers = clampInt(bounds.min_workers, 1, 64, base.bounds.min_workers);
  const maxWorkers = clampInt(bounds.max_workers, minWorkers, 64, base.bounds.max_workers);
  const allowedRisks = Array.isArray(safety.allowed_risks)
    ? safety.allowed_risks.map((x: unknown) => cleanText(x, 40).toLowerCase()).filter(Boolean)
    : base.safety.allowed_risks;
  const denyHabitIds = Array.isArray(safety.deny_habit_ids)
    ? safety.deny_habit_ids.map((x: unknown) => cleanText(x, 120)).filter(Boolean)
    : base.safety.deny_habit_ids;
  const runnerRaw = cleanText(execution.runner_path || base.execution.runner_path, 520) || base.execution.runner_path;

  return {
    version: cleanText(raw.version || base.version, 40) || base.version,
    enabled: raw.enabled === true,
    bounds: {
      min_workers: minWorkers,
      max_workers: maxWorkers
    },
    hysteresis: {
      scale_up_queue_threshold: clampInt(hysteresis.scale_up_queue_threshold, 1, 1000, base.hysteresis.scale_up_queue_threshold),
      scale_down_queue_threshold: clampInt(hysteresis.scale_down_queue_threshold, 0, 1000, base.hysteresis.scale_down_queue_threshold),
      cooldown_sec: clampInt(hysteresis.cooldown_sec, 0, 24 * 60 * 60, base.hysteresis.cooldown_sec)
    },
    safety: {
      allowed_risks: Array.from(new Set(allowedRisks.length ? allowedRisks : base.safety.allowed_risks)),
      deny_habit_ids: Array.from(new Set(denyHabitIds)),
      require_explicit_allow: safety.require_explicit_allow === true
    },
    execution: {
      runner_path: path.isAbsolute(runnerRaw) ? runnerRaw : path.join(ROOT, runnerRaw),
      apply_default: execution.apply_default === true,
      payload_json_default: cleanText(execution.payload_json_default || base.execution.payload_json_default, 20000) || '{}'
    },
    outputs: {
      state_path: resolvePath(outputs.state_path, base.outputs.state_path),
      latest_path: resolvePath(outputs.latest_path, base.outputs.latest_path),
      history_path: resolvePath(outputs.history_path, base.outputs.history_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function loadState(statePath: string) {
  const raw = readJson(statePath, {
    version: 1,
    last_workers: 1,
    cooldown_until: null,
    last_scale_at: null,
    last_reason: null,
    updated_at: null
  });
  return {
    version: 1,
    last_workers: clampInt(raw && raw.last_workers, 1, 64, 1),
    cooldown_until: cleanText(raw && raw.cooldown_until, 64) || null,
    last_scale_at: cleanText(raw && raw.last_scale_at, 64) || null,
    last_reason: cleanText(raw && raw.last_reason, 120) || null,
    updated_at: cleanText(raw && raw.updated_at, 64) || null
  };
}

function normalizeQueue(rows: unknown) {
  if (!Array.isArray(rows)) return [];
  return rows.map((row: AnyObj, index: number) => ({
    habit_id: cleanText(row && (row.habit_id || row.id || `habit_${index + 1}`), 120),
    risk: cleanText(row && row.risk, 40).toLowerCase() || 'low',
    allow_parallel: row && row.allow_parallel === true,
    enabled: row ? row.enabled !== false : true
  })).filter((row: AnyObj) => row.habit_id);
}

function gateQueue(queue: AnyObj[], policy: AnyObj) {
  const allowed = new Set((policy.safety.allowed_risks || []).map((x: unknown) => cleanText(x, 40).toLowerCase()));
  const deny = new Set((policy.safety.deny_habit_ids || []).map((x: unknown) => cleanText(x, 120)));
  const eligible: AnyObj[] = [];
  const blocked: AnyObj[] = [];

  for (const row of queue) {
    if (row.enabled !== true) {
      blocked.push({ habit_id: row.habit_id, gate: 'enabled', reason: 'habit_disabled' });
      continue;
    }
    if (deny.has(row.habit_id)) {
      blocked.push({ habit_id: row.habit_id, gate: 'denylist', reason: 'habit_explicitly_denied' });
      continue;
    }
    if (!allowed.has(String(row.risk || 'low').toLowerCase())) {
      blocked.push({ habit_id: row.habit_id, gate: 'risk', reason: 'risk_not_allowed', risk: row.risk });
      continue;
    }
    if (policy.safety.require_explicit_allow === true && row.allow_parallel !== true) {
      blocked.push({ habit_id: row.habit_id, gate: 'allow_parallel', reason: 'explicit_parallel_allow_required' });
      continue;
    }
    eligible.push(row);
  }

  return { eligible, blocked };
}

function pickTargetWorkers(eligibleCount: number, policy: AnyObj, state: AnyObj) {
  const minW = Number(policy.bounds.min_workers || 1);
  const maxW = Number(policy.bounds.max_workers || minW);
  const upThreshold = Number(policy.hysteresis.scale_up_queue_threshold || 1);
  const downThreshold = Number(policy.hysteresis.scale_down_queue_threshold || 0);
  const cooldownSec = Number(policy.hysteresis.cooldown_sec || 0);

  const baseline = clampInt(state.last_workers, minW, maxW, minW);
  let desired = baseline;
  let reason = 'steady';

  if (eligibleCount >= upThreshold) {
    desired = Math.min(maxW, Math.max(minW, Math.ceil(eligibleCount / Math.max(1, upThreshold))));
    reason = desired > baseline ? 'scale_up' : 'steady';
  } else if (eligibleCount <= downThreshold) {
    desired = minW;
    reason = desired < baseline ? 'scale_down' : 'steady';
  }

  const nowMs = Date.now();
  const cooldownUntilMs = Date.parse(String(state.cooldown_until || ''));
  const cooldownActive = Number.isFinite(cooldownUntilMs) && cooldownUntilMs > nowMs;

  if (cooldownActive && desired !== baseline) {
    return {
      workers: baseline,
      reason: 'cooldown_hold',
      scaled: false,
      cooldown_until: state.cooldown_until || null
    };
  }

  const scaled = desired !== baseline;
  const cooldownUntil = scaled && cooldownSec > 0
    ? new Date(nowMs + (cooldownSec * 1000)).toISOString()
    : state.cooldown_until || null;

  return {
    workers: desired,
    reason,
    scaled,
    cooldown_until: cooldownUntil
  };
}

async function runHabitCommand(runnerPath: string, habitId: string, payloadJson: string) {
  return new Promise((resolve) => {
    const child = spawn(process.execPath, [runnerPath, '--id', habitId, '--json', payloadJson], {
      cwd: ROOT,
      stdio: ['ignore', 'pipe', 'pipe']
    });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk: Buffer) => { stdout += String(chunk || ''); });
    child.stderr.on('data', (chunk: Buffer) => { stderr += String(chunk || ''); });
    child.on('close', (code: number) => {
      resolve({
        habit_id: habitId,
        ok: Number(code || 0) === 0,
        exit_code: Number(code || 0),
        stdout: stdout.slice(0, 800),
        stderr: stderr.slice(0, 800)
      });
    });
  });
}

async function runPool(habitIds: string[], workers: number, runnerPath: string, payloadJson: string) {
  const queue = habitIds.slice();
  const results: AnyObj[] = [];
  const active = new Set<Promise<void>>();

  async function launchNext() {
    if (!queue.length) return;
    const habitId = String(queue.shift() || '');
    if (!habitId) return;
    const task = runHabitCommand(runnerPath, habitId, payloadJson)
      .then((row: AnyObj) => { results.push(row); })
      .finally(() => { active.delete(task); }) as Promise<void>;
    active.add(task);
  }

  const width = Math.max(1, workers);
  for (let i = 0; i < width && queue.length; i += 1) await launchNext();

  while (active.size > 0) {
    await Promise.race(Array.from(active));
    while (active.size < width && queue.length) {
      await launchNext();
    }
  }

  return results.sort((a, b) => String(a.habit_id).localeCompare(String(b.habit_id)));
}

function planFromArgs(args: AnyObj, policyPath?: string) {
  const policy = loadPolicy(policyPath || (args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH));
  const queue = normalizeQueue(parseJsonArg(args['queue-json'] || args.queue_json || '', []));
  const state = loadState(policy.outputs.state_path);

  if (!policy.enabled && !toBool(args.force, false)) {
    return {
      ok: true,
      ts: nowIso(),
      type: 'habit_cell_pool_executor',
      result: 'disabled_by_policy',
      policy_path: rel(policy.policy_path)
    };
  }

  const gated = gateQueue(queue, policy);
  const target = pickTargetWorkers(gated.eligible.length, policy, state);

  state.last_workers = clampInt(target.workers, Number(policy.bounds.min_workers || 1), Number(policy.bounds.max_workers || 1), Number(policy.bounds.min_workers || 1));
  state.cooldown_until = target.cooldown_until;
  state.last_scale_at = target.scaled ? nowIso() : state.last_scale_at;
  state.last_reason = target.reason;
  state.updated_at = nowIso();
  writeJsonAtomic(policy.outputs.state_path, state);

  const out = {
    ok: true,
    ts: nowIso(),
    type: 'habit_cell_pool_executor_plan',
    metrics: {
      queued: queue.length,
      eligible: gated.eligible.length,
      blocked: gated.blocked.length,
      target_workers: target.workers
    },
    reason: target.reason,
    cooldown_until: target.cooldown_until,
    blocked: gated.blocked,
    eligible_habits: gated.eligible.map((row: AnyObj) => row.habit_id),
    policy_path: rel(policy.policy_path)
  };

  writeJsonAtomic(policy.outputs.latest_path, out);
  appendJsonl(policy.outputs.history_path, {
    ts: out.ts,
    type: out.type,
    queued: out.metrics.queued,
    eligible: out.metrics.eligible,
    blocked: out.metrics.blocked,
    target_workers: out.metrics.target_workers,
    reason: out.reason,
    ok: true
  });

  return out;
}

async function cmdExecute(args: AnyObj) {
  const strict = toBool(args.strict, true);
  const apply = toBool(args.apply, false);
  const plan = planFromArgs(args);
  if (!plan || plan.ok !== true) return plan;
  if (plan.result === 'disabled_by_policy') return { ...plan, strict };

  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  const payloadJson = cleanText(args['payload-json'] || args.payload_json || policy.execution.payload_json_default || '{}', 20000) || '{}';

  if (!apply) {
    return {
      ok: true,
      ts: nowIso(),
      type: 'habit_cell_pool_executor_execute',
      strict,
      dry_run: true,
      workers: plan.metrics.target_workers,
      eligible_habits: plan.eligible_habits,
      commands: (plan.eligible_habits || []).map((habitId: string) => [process.execPath, rel(policy.execution.runner_path), '--id', habitId, '--json', payloadJson]),
      blocked: plan.blocked,
      policy_path: rel(policy.policy_path)
    };
  }

  const results = await runPool(plan.eligible_habits || [], Number(plan.metrics.target_workers || 1), policy.execution.runner_path, payloadJson);
  const failures = results.filter((row: AnyObj) => row.ok !== true);

  const out = {
    ok: failures.length === 0,
    ts: nowIso(),
    type: 'habit_cell_pool_executor_execute',
    strict,
    dry_run: false,
    workers: plan.metrics.target_workers,
    eligible_habits: plan.eligible_habits,
    blocked: plan.blocked,
    failures: failures.length,
    results,
    policy_path: rel(policy.policy_path)
  };

  writeJsonAtomic(policy.outputs.latest_path, out);
  appendJsonl(policy.outputs.history_path, {
    ts: out.ts,
    type: out.type,
    failures: out.failures,
    workers: out.workers,
    eligible: Array.isArray(out.eligible_habits) ? out.eligible_habits.length : 0,
    blocked: Array.isArray(out.blocked) ? out.blocked.length : 0,
    ok: out.ok
  });

  return out;
}

function cmdStatus(args: AnyObj) {
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  return {
    ok: true,
    ts: nowIso(),
    type: 'habit_cell_pool_executor_status',
    policy_path: rel(policy.policy_path),
    state_path: rel(policy.outputs.state_path),
    state: readJson(policy.outputs.state_path, null),
    latest: readJson(policy.outputs.latest_path, null)
  };
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/habits/habit_cell_pool_executor.js plan --queue-json="[{\"id\":\"habit_a\"}]"');
  console.log('  node systems/habits/habit_cell_pool_executor.js execute --queue-json="[{\"id\":\"habit_a\"}]" [--apply=1|0] [--strict=1|0]');
  console.log('  node systems/habits/habit_cell_pool_executor.js status');
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = String(args._[0] || 'status').toLowerCase();
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') { usage(); return; }

  const payload = cmd === 'plan' ? planFromArgs(args)
    : cmd === 'execute' ? await cmdExecute(args)
      : cmd === 'status' ? cmdStatus(args)
        : { ok: false, error: `unknown_command:${cmd}` };

  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
  if (payload && payload.ok === false && toBool(args.strict, true)) process.exit(1);
  if (payload && payload.ok === false) process.exit(1);
}

if (require.main === module) {
  main().catch((err) => {
    process.stdout.write(`${JSON.stringify({ ok: false, error: cleanText((err as AnyObj)?.message || err || 'habit_cell_pool_executor_failed', 260) })}\n`);
    process.exit(1);
  });
}

module.exports = { loadPolicy, planFromArgs, cmdStatus, cmdExecute };
