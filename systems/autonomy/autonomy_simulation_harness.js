#!/usr/bin/env node
'use strict';

/**
 * autonomy_simulation_harness.js
 *
 * End-to-end replay-style health simulation over recent autonomy activity.
 * Produces a deterministic scorecard for drift, yield, and safety gates.
 *
 * Usage:
 *   node systems/autonomy/autonomy_simulation_harness.js run [YYYY-MM-DD] [--days=N] [--write=1|0] [--strict]
 *   node systems/autonomy/autonomy_simulation_harness.js status [YYYY-MM-DD] [--days=N]
 */

const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..', '..');
const RUNS_DIR = process.env.AUTONOMY_SIM_RUNS_DIR
  ? path.resolve(String(process.env.AUTONOMY_SIM_RUNS_DIR))
  : path.join(ROOT, 'state', 'autonomy', 'runs');
const PROPOSALS_DIR = process.env.AUTONOMY_SIM_PROPOSALS_DIR
  ? path.resolve(String(process.env.AUTONOMY_SIM_PROPOSALS_DIR))
  : path.join(ROOT, 'state', 'sensory', 'proposals');
const OUTPUT_DIR = process.env.AUTONOMY_SIM_OUTPUT_DIR
  ? path.resolve(String(process.env.AUTONOMY_SIM_OUTPUT_DIR))
  : path.join(ROOT, 'state', 'autonomy', 'simulations');

const DRIFT_WARN = Number(process.env.AUTONOMY_SIM_DRIFT_WARN || 0.65);
const DRIFT_FAIL = Number(process.env.AUTONOMY_SIM_DRIFT_FAIL || 0.85);
const YIELD_WARN = Number(process.env.AUTONOMY_SIM_YIELD_WARN || 0.2);
const YIELD_FAIL = Number(process.env.AUTONOMY_SIM_YIELD_FAIL || 0.08);
const SAFETY_WARN = Number(process.env.AUTONOMY_SIM_SAFETY_WARN || 0.25);
const SAFETY_FAIL = Number(process.env.AUTONOMY_SIM_SAFETY_FAIL || 0.45);
const MIN_ATTEMPTS = Math.max(1, Number(process.env.AUTONOMY_SIM_MIN_ATTEMPTS || 5));
const MAX_WINDOW_DAYS = Math.max(1, Math.floor(Number(process.env.AUTONOMY_SIM_MAX_DAYS || 180)));

function usage() {
  console.log('Usage:');
  console.log('  node systems/autonomy/autonomy_simulation_harness.js run [YYYY-MM-DD] [--days=N] [--write=1|0] [--strict]');
  console.log('  node systems/autonomy/autonomy_simulation_harness.js status [YYYY-MM-DD] [--days=N]');
}

function parseArgs(argv) {
  const out = { _: [] };
  for (const arg of argv) {
    if (!arg.startsWith('--')) {
      out._.push(arg);
      continue;
    }
    const eq = arg.indexOf('=');
    if (eq === -1) out[arg.slice(2)] = true;
    else out[arg.slice(2, eq)] = arg.slice(eq + 1);
  }
  return out;
}

function isDateStr(v) {
  return /^\d{4}-\d{2}-\d{2}$/.test(String(v || ''));
}

function todayStr() {
  return new Date().toISOString().slice(0, 10);
}

function resolveDate(args) {
  const first = String(args._[1] || '').trim();
  if (isDateStr(first)) return first;
  const second = String(args._[0] || '').trim();
  if (isDateStr(second)) return second;
  return todayStr();
}

function toInt(v, fallback, lo = 1, hi = 365) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  const i = Math.floor(n);
  if (i < lo) return lo;
  if (i > hi) return hi;
  return i;
}

function ensureDir(dirPath) {
  if (!fs.existsSync(dirPath)) fs.mkdirSync(dirPath, { recursive: true });
}

function readJson(fp, fallback) {
  try {
    if (!fs.existsSync(fp)) return fallback;
    return JSON.parse(fs.readFileSync(fp, 'utf8'));
  } catch {
    return fallback;
  }
}

function readJsonl(fp) {
  if (!fs.existsSync(fp)) return [];
  return fs.readFileSync(fp, 'utf8')
    .split('\n')
    .filter(Boolean)
    .map((line) => {
      try { return JSON.parse(line); } catch { return null; }
    })
    .filter(Boolean);
}

function dateWindow(endDateStr, days) {
  const end = new Date(`${endDateStr}T00:00:00.000Z`);
  if (Number.isNaN(end.getTime())) return [];
  const out = [];
  for (let i = 0; i < days; i++) {
    const d = new Date(end);
    d.setUTCDate(end.getUTCDate() - i);
    out.push(d.toISOString().slice(0, 10));
  }
  return out;
}

function safeRate(num, den) {
  const n = Number(num || 0);
  const d = Number(den || 0);
  if (!Number.isFinite(n) || !Number.isFinite(d) || d <= 0) return 0;
  return n / d;
}

function isAttemptRun(evt) {
  if (!evt || evt.type !== 'autonomy_run') return false;
  const result = String(evt.result || '');
  if (!result) return false;
  if (result === 'lock_busy' || result === 'stop_repeat_gate_interval') return false;
  return true;
}

function isNoProgress(evt) {
  if (!evt || evt.type !== 'autonomy_run') return false;
  if (evt.result === 'executed') return String(evt.outcome || '') !== 'shipped';
  const result = String(evt.result || '');
  return result.startsWith('stop_') || result.startsWith('init_gate_');
}

function isSafetyStop(evt) {
  if (!evt || evt.type !== 'autonomy_run') return false;
  const result = String(evt.result || '');
  return result.includes('human_escalation')
    || result.includes('tier1_governance')
    || result.includes('medium_risk_guard')
    || result.includes('capability_cooldown')
    || result.includes('directive_pulse_tier_reservation');
}

function objectiveIdFromRun(evt) {
  if (!evt || evt.type !== 'autonomy_run') return '';
  const pulse = evt.directive_pulse && typeof evt.directive_pulse === 'object' ? evt.directive_pulse : {};
  const id = String(pulse.objective_id || evt.objective_id || '').trim();
  return id;
}

function queueSnapshotForWindow(dates) {
  let total = 0;
  let pending = 0;
  let stalePending = 0;
  const nowMs = Date.now();
  for (const d of dates) {
    const fp = path.join(PROPOSALS_DIR, `${d}.json`);
    const raw = readJson(fp, []);
    const rows = Array.isArray(raw) ? raw : (raw && Array.isArray(raw.proposals) ? raw.proposals : []);
    for (const row of rows) {
      if (!row || typeof row !== 'object') continue;
      total += 1;
      const status = String(row.status || row.state || 'pending').trim().toLowerCase();
      if (status === 'pending' || status === 'open') {
        pending += 1;
        const ms = Date.parse(`${d}T00:00:00.000Z`);
        const ageHours = Number.isFinite(ms) ? Math.max(0, (nowMs - ms) / 3600000) : 0;
        if (ageHours >= 72) stalePending += 1;
      }
    }
  }
  return { total, pending, stale_pending_72h: stalePending };
}

function computeSimulation(endDateStr, days) {
  const dates = dateWindow(endDateStr, days);
  const runs = [];
  for (const d of dates) {
    runs.push(...readJsonl(path.join(RUNS_DIR, `${d}.jsonl`)));
  }
  const runRows = runs.filter((row) => row && row.type === 'autonomy_run');
  const attempts = runRows.filter(isAttemptRun);
  const executed = runRows.filter((row) => row && row.result === 'executed');
  const shipped = executed.filter((row) => String(row.outcome || '') === 'shipped');
  const noProgress = attempts.filter(isNoProgress);
  const safetyStops = attempts.filter(isSafetyStop);

  const objectiveCounts = {};
  for (const row of executed) {
    const id = objectiveIdFromRun(row);
    if (!id) continue;
    objectiveCounts[id] = Number(objectiveCounts[id] || 0) + 1;
  }

  const driftRate = safeRate(noProgress.length, attempts.length);
  const yieldRate = safeRate(shipped.length, executed.length);
  const safetyRate = safeRate(safetyStops.length, attempts.length);

  const queue = queueSnapshotForWindow(dates);

  const checks = {
    drift_rate: {
      value: Number(driftRate.toFixed(3)),
      warn: DRIFT_WARN,
      fail: DRIFT_FAIL,
      status: driftRate >= DRIFT_FAIL ? 'fail' : driftRate >= DRIFT_WARN ? 'warn' : 'pass'
    },
    yield_rate: {
      value: Number(yieldRate.toFixed(3)),
      warn: YIELD_WARN,
      fail: YIELD_FAIL,
      status: yieldRate <= YIELD_FAIL ? 'fail' : yieldRate <= YIELD_WARN ? 'warn' : 'pass'
    },
    safety_stop_rate: {
      value: Number(safetyRate.toFixed(3)),
      warn: SAFETY_WARN,
      fail: SAFETY_FAIL,
      status: safetyRate >= SAFETY_FAIL ? 'fail' : safetyRate >= SAFETY_WARN ? 'warn' : 'pass'
    },
    attempt_volume: {
      value: attempts.length,
      min: MIN_ATTEMPTS,
      status: attempts.length < MIN_ATTEMPTS ? 'warn' : 'pass'
    }
  };

  const failingChecks = Object.entries(checks)
    .filter(([, row]) => row && row.status === 'fail')
    .map(([id]) => id);
  const warningChecks = Object.entries(checks)
    .filter(([, row]) => row && row.status === 'warn')
    .map(([id]) => id);

  const verdict = failingChecks.length > 0
    ? 'fail'
    : warningChecks.length > 0
      ? 'warn'
      : 'pass';

  const recommendations = [];
  if (checks.drift_rate.status !== 'pass') {
    recommendations.push('Increase proposal quality floor or tighten objective binding for non-executable proposals.');
  }
  if (checks.yield_rate.status !== 'pass') {
    recommendations.push('Bias selection toward high-value, executable proposals and reduce medium-risk capacity until shipped rate recovers.');
  }
  if (queue.pending > 80 || queue.stale_pending_72h > 10) {
    recommendations.push('Run proposal queue SLO drain to park stale backlog and reduce queue pressure.');
  }

  return {
    ok: true,
    type: 'autonomy_simulation_harness',
    ts: new Date().toISOString(),
    end_date: endDateStr,
    days,
    verdict,
    checks,
    counters: {
      run_rows: runRows.length,
      attempts: attempts.length,
      executed: executed.length,
      shipped: shipped.length,
      no_progress: noProgress.length,
      safety_stops: safetyStops.length
    },
    queue,
    objective_mix: {
      executed_total: executed.length,
      objective_count: Object.keys(objectiveCounts).length,
      counts: objectiveCounts
    },
    recommendations: recommendations.slice(0, 5)
  };
}

function writeOutput(dateStr, payload) {
  ensureDir(OUTPUT_DIR);
  const fp = path.join(OUTPUT_DIR, `${dateStr}.json`);
  fs.writeFileSync(fp, JSON.stringify(payload, null, 2) + '\n', 'utf8');
  return fp;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = String(args._[0] || 'run').trim().toLowerCase();
  if (cmd === '--help' || cmd === '-h' || cmd === 'help') {
    usage();
    return;
  }

  const dateStr = resolveDate(args);
  const days = toInt(args.days, 14, 1, MAX_WINDOW_DAYS);
  const strict = args.strict === true;
  const write = String(args.write == null ? '1' : args.write).trim() !== '0';
  const payload = computeSimulation(dateStr, days);
  if (write && (cmd === 'run' || cmd === 'status')) {
    payload.report_path = writeOutput(dateStr, payload);
  }
  process.stdout.write(JSON.stringify(payload, null, 2) + '\n');
  if (strict && payload.verdict === 'fail') process.exit(2);
}

if (require.main === module) {
  main();
}

module.exports = {
  computeSimulation,
  queueSnapshotForWindow
};
