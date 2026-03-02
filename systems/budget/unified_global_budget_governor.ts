#!/usr/bin/env node
'use strict';
export {};

/**
 * BL-023
 * Unified global budget governor for reflex/autonomy/focus/dream/spawn lanes.
 *
 * Usage:
 *   node systems/budget/unified_global_budget_governor.js evaluate --module=autonomy --tokens=120 [--date=YYYY-MM-DD] [--apply=1|0] [--strict=1|0]
 *   node systems/budget/unified_global_budget_governor.js status [--date=YYYY-MM-DD]
 */

const fs = require('fs');
const path = require('path');

type AnyObj = Record<string, any>;

const ROOT = process.env.UNIFIED_BUDGET_GOVERNOR_ROOT
  ? path.resolve(process.env.UNIFIED_BUDGET_GOVERNOR_ROOT)
  : path.resolve(__dirname, '..', '..');

const DEFAULT_POLICY_PATH = process.env.UNIFIED_BUDGET_GOVERNOR_POLICY_PATH
  ? path.resolve(process.env.UNIFIED_BUDGET_GOVERNOR_POLICY_PATH)
  : path.join(ROOT, 'config', 'unified_global_budget_governor_policy.json');

function nowIso() { return new Date().toISOString(); }
function cleanText(v: unknown, maxLen = 360) { return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen); }
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
function clampNumber(v: unknown, lo: number, hi: number, fallback: number) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  if (n < lo) return lo;
  if (n > hi) return hi;
  return n;
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

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    modules: ['reflex', 'autonomy', 'focus', 'dream', 'spawn'],
    module_daily_caps: {
      reflex: 1200,
      autonomy: 4500,
      focus: 1200,
      dream: 1500,
      spawn: 1500
    },
    daily_token_cap_total: 9000,
    contention: {
      degrade_at_ratio: 0.85,
      deny_at_ratio: 1
    },
    outputs: {
      state_path: 'state/budget/unified_global_budget_governor/state.json',
      decisions_path: 'state/budget/unified_global_budget_governor/decisions.jsonl',
      latest_path: 'state/budget/unified_global_budget_governor/latest.json',
      history_path: 'state/budget/unified_global_budget_governor/history.jsonl',
      autopause_path: 'state/autonomy/budget_autopause.json'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const base = defaultPolicy();
  const raw = readJson(policyPath, {});
  const contention = raw.contention && typeof raw.contention === 'object' ? raw.contention : {};
  const outputs = raw.outputs && typeof raw.outputs === 'object' ? raw.outputs : {};
  const modules = Array.isArray(raw.modules)
    ? raw.modules.map((x: unknown) => cleanText(x, 80)).filter(Boolean)
    : base.modules;

  const moduleCaps: AnyObj = { ...base.module_daily_caps };
  const rawCaps = raw.module_daily_caps && typeof raw.module_daily_caps === 'object' ? raw.module_daily_caps : {};
  for (const moduleName of modules) {
    moduleCaps[moduleName] = clampInt(rawCaps[moduleName], 1, 100000000, Number(base.module_daily_caps[moduleName] || base.daily_token_cap_total));
  }

  return {
    version: cleanText(raw.version || base.version, 40) || base.version,
    enabled: raw.enabled !== false,
    modules: Array.from(new Set(modules)),
    module_daily_caps: moduleCaps,
    daily_token_cap_total: clampInt(raw.daily_token_cap_total, 1, 100000000, base.daily_token_cap_total),
    contention: {
      degrade_at_ratio: clampNumber(contention.degrade_at_ratio, 0, 1, base.contention.degrade_at_ratio),
      deny_at_ratio: clampNumber(contention.deny_at_ratio, 0.1, 2, base.contention.deny_at_ratio)
    },
    outputs: {
      state_path: resolvePath(outputs.state_path, base.outputs.state_path),
      decisions_path: resolvePath(outputs.decisions_path, base.outputs.decisions_path),
      latest_path: resolvePath(outputs.latest_path, base.outputs.latest_path),
      history_path: resolvePath(outputs.history_path, base.outputs.history_path),
      autopause_path: resolvePath(outputs.autopause_path, base.outputs.autopause_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function loadState(statePath: string) {
  const raw = readJson(statePath, {
    version: 1,
    updated_at: null,
    by_day: {}
  });
  const byDay = raw && raw.by_day && typeof raw.by_day === 'object' ? raw.by_day : {};
  return {
    version: 1,
    updated_at: cleanText(raw && raw.updated_at, 64) || null,
    by_day: byDay
  };
}

function ensureDay(state: AnyObj, dateKey: string, modules: string[]) {
  if (!state.by_day[dateKey] || typeof state.by_day[dateKey] !== 'object') state.by_day[dateKey] = {};
  for (const moduleName of modules) {
    if (!Number.isFinite(Number(state.by_day[dateKey][moduleName]))) state.by_day[dateKey][moduleName] = 0;
  }
}

function totalForDay(dayRow: AnyObj, modules: string[]) {
  return modules.reduce((sum: number, moduleName: string) => sum + Number(dayRow[moduleName] || 0), 0);
}

function setAutopause(pathOut: string, active: boolean, reason: string) {
  const payload = {
    active,
    updated_at: nowIso(),
    reason: cleanText(reason, 200)
  };
  writeJsonAtomic(pathOut, payload);
}

function cmdEvaluate(args: AnyObj) {
  const strict = toBool(args.strict, true);
  const apply = toBool(args.apply, true);
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  if (!policy.enabled) return { ok: true, strict, result: 'disabled_by_policy', policy_path: rel(policy.policy_path) };

  const moduleName = cleanText(args.module, 80);
  if (!policy.modules.includes(moduleName)) {
    return { ok: false, ts: nowIso(), type: 'unified_global_budget_governor', strict, error: 'unknown_module', module: moduleName, policy_path: rel(policy.policy_path) };
  }

  const tokens = clampInt(args.tokens, 0, 100000000, 0);
  const dateKey = cleanText(args.date, 20) || new Date().toISOString().slice(0, 10);
  const state = loadState(policy.outputs.state_path);
  ensureDay(state, dateKey, policy.modules);

  const day = state.by_day[dateKey];
  const currentModule = Number(day[moduleName] || 0);
  const currentTotal = totalForDay(day, policy.modules);
  const projectedModule = currentModule + tokens;
  const projectedTotal = currentTotal + tokens;

  const moduleCap = Number(policy.module_daily_caps[moduleName] || policy.daily_token_cap_total);
  const totalCap = Number(policy.daily_token_cap_total || 1);
  const projectedRatio = totalCap > 0 ? projectedTotal / totalCap : 1;

  const blockers: AnyObj[] = [];
  let decision = 'allow';

  if (projectedModule > moduleCap) {
    blockers.push({ gate: 'module_cap', reason: 'module_daily_cap_exceeded', module: moduleName, projected: projectedModule, cap: moduleCap });
    decision = 'deny';
  }
  if (projectedTotal > totalCap || projectedRatio >= Number(policy.contention.deny_at_ratio || 1)) {
    blockers.push({ gate: 'global_cap', reason: 'global_daily_cap_exceeded', projected: projectedTotal, cap: totalCap, ratio: Number(projectedRatio.toFixed(6)) });
    decision = 'deny';
  } else if (decision !== 'deny' && projectedRatio >= Number(policy.contention.degrade_at_ratio || 0.85)) {
    decision = 'degrade';
  }

  if (apply && decision !== 'deny') {
    day[moduleName] = projectedModule;
    state.updated_at = nowIso();
    writeJsonAtomic(policy.outputs.state_path, state);
    if (decision === 'allow') setAutopause(policy.outputs.autopause_path, false, 'budget_within_limits');
  }

  if (apply && decision === 'deny') {
    setAutopause(policy.outputs.autopause_path, true, blockers.map((b: AnyObj) => String(b.reason || b.gate || 'budget_block')).join(','));
  }

  const out = {
    ok: decision !== 'deny',
    ts: nowIso(),
    type: 'unified_global_budget_governor',
    strict,
    apply,
    module: moduleName,
    decision,
    tokens,
    metrics: {
      date: dateKey,
      current_module_tokens: currentModule,
      projected_module_tokens: projectedModule,
      module_cap: moduleCap,
      current_total_tokens: currentTotal,
      projected_total_tokens: projectedTotal,
      total_cap: totalCap,
      projected_total_ratio: Number(projectedRatio.toFixed(6))
    },
    blockers,
    shared_state_path: rel(policy.outputs.state_path),
    policy_path: rel(policy.policy_path)
  };

  writeJsonAtomic(policy.outputs.latest_path, out);
  appendJsonl(policy.outputs.decisions_path, {
    ts: out.ts,
    type: out.type,
    module: out.module,
    decision: out.decision,
    tokens: out.tokens,
    projected_total_ratio: out.metrics.projected_total_ratio,
    ok: out.ok
  });
  appendJsonl(policy.outputs.history_path, {
    ts: out.ts,
    type: out.type,
    module: out.module,
    decision: out.decision,
    blockers: out.blockers.map((row: AnyObj) => row.gate),
    ok: out.ok
  });

  return out;
}

function cmdStatus(args: AnyObj) {
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  const state = loadState(policy.outputs.state_path);
  const dateKey = cleanText(args.date, 20) || new Date().toISOString().slice(0, 10);
  ensureDay(state, dateKey, policy.modules);

  return {
    ok: true,
    ts: nowIso(),
    type: 'unified_global_budget_governor_status',
    date: dateKey,
    modules: policy.modules,
    usage: state.by_day[dateKey],
    total: totalForDay(state.by_day[dateKey], policy.modules),
    policy_path: rel(policy.policy_path),
    latest: readJson(policy.outputs.latest_path, null)
  };
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/budget/unified_global_budget_governor.js evaluate --module=autonomy --tokens=120 [--date=YYYY-MM-DD] [--apply=1|0] [--strict=1|0]');
  console.log('  node systems/budget/unified_global_budget_governor.js status [--date=YYYY-MM-DD]');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = String(args._[0] || 'status').toLowerCase();
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') { usage(); return; }
  const payload = cmd === 'evaluate' ? cmdEvaluate(args)
    : cmd === 'status' ? cmdStatus(args)
      : { ok: false, error: `unknown_command:${cmd}` };
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
  if (payload.ok === false && toBool(args.strict, true)) process.exit(1);
  if (payload.ok === false) process.exit(1);
}

if (require.main === module) {
  try { main(); } catch (err) {
    process.stdout.write(`${JSON.stringify({ ok: false, error: cleanText((err as AnyObj)?.message || err || 'unified_global_budget_governor_failed', 260) })}\n`);
    process.exit(1);
  }
}

module.exports = { loadPolicy, cmdEvaluate, cmdStatus };
