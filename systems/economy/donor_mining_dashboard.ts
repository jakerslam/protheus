#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-199
 * Donor mining dashboard + CLI surface (`protheusctl mine dashboard`).
 */

const fs = require('fs');
const path = require('path');
const {
  ROOT,
  nowIso,
  cleanText,
  toBool,
  clampInt,
  clampNumber,
  parseArgs,
  readJson,
  readJsonl,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

type AnyObj = Record<string, any>;

const DEFAULT_POLICY_PATH = process.env.DONOR_MINING_DASHBOARD_POLICY_PATH
  ? path.resolve(process.env.DONOR_MINING_DASHBOARD_POLICY_PATH)
  : path.join(ROOT, 'config', 'donor_mining_dashboard_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/economy/donor_mining_dashboard.js dashboard [--donor=<id>] [--human=1] [--strict=1] [--policy=<path>]');
  console.log('  node systems/economy/donor_mining_dashboard.js rollback --reason=<text> [--policy=<path>]');
  console.log('  node systems/economy/donor_mining_dashboard.js status [--policy=<path>]');
}

function rel(filePath: string) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    flops_per_gpu_hour: 4.2e15,
    reward_units_per_gpu_hour: 1,
    settled_reward_ratio_default: 0.82,
    projection: {
      short_days: 7,
      mid_days: 30,
      long_days: 90,
      low_factor: 0.8,
      high_factor: 1.2
    },
    accepted_statuses: ['validated', 'applied', 'settled'],
    paths: {
      contributions_path: 'state/economy/contributions.json',
      donor_state_path: 'state/economy/donor_state.json',
      receipts_path: 'state/economy/receipts.jsonl',
      ledger_path: 'state/economy/tithe_ledger.jsonl',
      latest_path: 'state/economy/mining_dashboard/latest.json',
      history_path: 'state/economy/mining_dashboard/history.jsonl',
      rollback_log_path: 'state/economy/mining_dashboard/rollbacks.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const proj = raw.projection && typeof raw.projection === 'object' ? raw.projection : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 40) || base.version,
    enabled: raw.enabled !== false,
    flops_per_gpu_hour: clampNumber(raw.flops_per_gpu_hour, 1, 1e20, base.flops_per_gpu_hour),
    reward_units_per_gpu_hour: clampNumber(raw.reward_units_per_gpu_hour, 0, 1e9, base.reward_units_per_gpu_hour),
    settled_reward_ratio_default: clampNumber(
      raw.settled_reward_ratio_default,
      0,
      1,
      base.settled_reward_ratio_default
    ),
    projection: {
      short_days: clampInt(proj.short_days, 1, 3650, base.projection.short_days),
      mid_days: clampInt(proj.mid_days, 1, 3650, base.projection.mid_days),
      long_days: clampInt(proj.long_days, 1, 3650, base.projection.long_days),
      low_factor: clampNumber(proj.low_factor, 0, 10, base.projection.low_factor),
      high_factor: clampNumber(proj.high_factor, 0, 10, base.projection.high_factor)
    },
    accepted_statuses: Array.isArray(raw.accepted_statuses) && raw.accepted_statuses.length
      ? raw.accepted_statuses.map((v: unknown) => cleanText(v, 80).toLowerCase()).filter(Boolean)
      : base.accepted_statuses,
    paths: {
      contributions_path: resolvePath(paths.contributions_path, base.paths.contributions_path),
      donor_state_path: resolvePath(paths.donor_state_path, base.paths.donor_state_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path),
      ledger_path: resolvePath(paths.ledger_path, base.paths.ledger_path),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path),
      rollback_log_path: resolvePath(paths.rollback_log_path, base.paths.rollback_log_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function parseIsoMs(v: unknown) {
  const ms = Date.parse(String(v || ''));
  return Number.isFinite(ms) ? ms : null;
}

function loadContributions(policy: AnyObj) {
  const rows = readJson(policy.paths.contributions_path, []);
  return Array.isArray(rows) ? rows : [];
}

function loadDonorState(policy: AnyObj) {
  const row = readJson(policy.paths.donor_state_path, {});
  return row && typeof row === 'object' ? row : {};
}

function findLatestRollback(policy: AnyObj) {
  const rows = readJsonl(policy.paths.rollback_log_path).filter((row: any) => row && typeof row === 'object');
  if (!rows.length) return null;
  return rows[rows.length - 1];
}

function receiptsByDonor(policy: AnyObj) {
  const out: Record<string, number> = {};
  const rows = readJsonl(policy.paths.receipts_path).filter((row: any) => row && typeof row === 'object');
  for (const row of rows) {
    const payload = row.payload && typeof row.payload === 'object' ? row.payload : {};
    const donorId = cleanText(payload.donor_id || row.donor_id, 120);
    if (!donorId) continue;
    out[donorId] = Number(out[donorId] || 0) + 1;
  }
  return out;
}

function contributionsByDonor(rows: AnyObj[]) {
  const out: Record<string, AnyObj[]> = {};
  for (const row of rows) {
    if (!row || typeof row !== 'object') continue;
    const donorId = cleanText(row.donor_id || row.donor, 120);
    if (!donorId) continue;
    if (!out[donorId]) out[donorId] = [];
    out[donorId].push(row);
  }
  return out;
}

function sumGpuHours(rows: AnyObj[]) {
  return Number(rows
    .reduce((acc, row) => acc + Math.max(0, Number(row && row.gpu_hours || row && row.hours || 0)), 0)
    .toFixed(6));
}

function selectAcceptedRows(rows: AnyObj[], acceptedStatuses: string[]) {
  const accepted = new Set((acceptedStatuses || []).map((s) => cleanText(s, 80).toLowerCase()));
  return rows.filter((row) => accepted.has(cleanText(row && row.status, 80).toLowerCase()));
}

function computeDailyRecentHours(rows: AnyObj[], lookbackDays: number) {
  const nowMs = Date.now();
  const minMs = nowMs - (lookbackDays * 86400000);
  let total = 0;
  for (const row of rows) {
    const ts = parseIsoMs(row && (row.status_updated_at || row.received_at || row.ts));
    if (ts == null || ts < minMs) continue;
    total += Math.max(0, Number(row.gpu_hours || row.hours || 0));
  }
  return Number(total.toFixed(6));
}

function projectionBands(policy: AnyObj, acceptedRows: AnyObj[]) {
  const p = policy.projection || {};
  const lookbackDays = Math.max(1, Number(p.short_days || 7));
  const recentGpu = computeDailyRecentHours(acceptedRows, lookbackDays);
  const daily = recentGpu / lookbackDays;
  const lowFactor = clampNumber(p.low_factor, 0, 10, 0.8);
  const highFactor = clampNumber(p.high_factor, 0, 10, 1.2);

  const flops = (days: number, factor: number) => {
    const gpuHours = daily * days * factor;
    return Number((gpuHours * Number(policy.flops_per_gpu_hour || 0)).toFixed(3));
  };

  return {
    assumptions: {
      lookback_days: lookbackDays,
      avg_daily_gpu_hours: Number(daily.toFixed(6)),
      flops_per_gpu_hour: Number(policy.flops_per_gpu_hour || 0)
    },
    short: {
      days: Number(p.short_days || 7),
      low: flops(Number(p.short_days || 7), lowFactor),
      mid: flops(Number(p.short_days || 7), 1),
      high: flops(Number(p.short_days || 7), highFactor)
    },
    mid: {
      days: Number(p.mid_days || 30),
      low: flops(Number(p.mid_days || 30), lowFactor),
      mid: flops(Number(p.mid_days || 30), 1),
      high: flops(Number(p.mid_days || 30), highFactor)
    },
    long: {
      days: Number(p.long_days || 90),
      low: flops(Number(p.long_days || 90), lowFactor),
      mid: flops(Number(p.long_days || 90), 1),
      high: flops(Number(p.long_days || 90), highFactor)
    }
  };
}

function buildDashboard(policy: AnyObj, args: AnyObj) {
  if (policy.enabled !== true) {
    return {
      ok: true,
      type: 'donor_mining_dashboard',
      ts: nowIso(),
      result: 'disabled_by_policy'
    };
  }

  const contributions = loadContributions(policy);
  const donorState = loadDonorState(policy);
  const byDonor = contributionsByDonor(contributions);
  const receiptCounts = receiptsByDonor(policy);
  const donorIds = new Set<string>([
    ...Object.keys(byDonor),
    ...Object.keys(donorState)
  ]);

  const filterDonor = cleanText(args.donor || args.donor_id, 120);
  const rows: AnyObj[] = [];

  for (const donorId of donorIds) {
    if (filterDonor && donorId !== filterDonor) continue;
    const donorRows = byDonor[donorId] || [];
    const acceptedRows = selectAcceptedRows(donorRows, policy.accepted_statuses);
    const donorModel = donorState[donorId] && typeof donorState[donorId] === 'object' ? donorState[donorId] : {};

    const acceptedGpuHours = Number(donorModel.total_validated_gpu_hours || sumGpuHours(acceptedRows) || 0);
    const donorFlops = Number((acceptedGpuHours * Number(policy.flops_per_gpu_hour || 0)).toFixed(3));
    const acceptedWorkUnits = acceptedRows.length;
    const accruedRewards = Number((acceptedGpuHours * Number(policy.reward_units_per_gpu_hour || 0)).toFixed(6));

    const receiptCount = Number(receiptCounts[donorId] || 0);
    const settledRatio = acceptedWorkUnits > 0
      ? clampNumber(receiptCount / acceptedWorkUnits, 0, 1, policy.settled_reward_ratio_default)
      : clampNumber(policy.settled_reward_ratio_default, 0, 1, 0.82);

    const settledRewards = Number((accruedRewards * settledRatio).toFixed(6));

    rows.push({
      donor_id: donorId,
      donor_flops: donorFlops,
      accepted_work_units: acceptedWorkUnits,
      accrued_rewards: accruedRewards,
      settled_rewards: settledRewards,
      reward_settlement_ratio: Number(settledRatio.toFixed(6)),
      projection_bands: projectionBands(policy, acceptedRows)
    });
  }

  const totals = rows.reduce((acc, row) => {
    acc.donor_flops += Number(row.donor_flops || 0);
    acc.accepted_work_units += Number(row.accepted_work_units || 0);
    acc.accrued_rewards += Number(row.accrued_rewards || 0);
    acc.settled_rewards += Number(row.settled_rewards || 0);
    return acc;
  }, {
    donor_flops: 0,
    accepted_work_units: 0,
    accrued_rewards: 0,
    settled_rewards: 0
  });

  const latestRollback = findLatestRollback(policy);

  return {
    ok: true,
    type: 'donor_mining_dashboard',
    lane_id: 'V3-RACE-199',
    ts: nowIso(),
    dashboard_receipt_id: `mine_dash_${stableHash(JSON.stringify({ rows, totals, latestRollback }), 14)}`,
    donor_count: rows.length,
    donors: rows.sort((a, b) => String(a.donor_id).localeCompare(String(b.donor_id))),
    totals: {
      donor_flops: Number(totals.donor_flops.toFixed(3)),
      accepted_work_units: Number(totals.accepted_work_units || 0),
      accrued_rewards: Number(totals.accrued_rewards.toFixed(6)),
      settled_rewards: Number(totals.settled_rewards.toFixed(6))
    },
    rollback_visibility: {
      last_rollback_event: latestRollback,
      rollback_log_path: rel(policy.paths.rollback_log_path)
    }
  };
}

function printHuman(payload: AnyObj) {
  const donors = Array.isArray(payload && payload.donors) ? payload.donors : [];
  console.log(`Donor Mining Dashboard (${donors.length} donors)`);
  console.log(`Flops: ${Number(payload?.totals?.donor_flops || 0).toFixed(3)} | Work units: ${Number(payload?.totals?.accepted_work_units || 0)}`);
  console.log(`Rewards: accrued=${Number(payload?.totals?.accrued_rewards || 0).toFixed(6)} settled=${Number(payload?.totals?.settled_rewards || 0).toFixed(6)}`);
  if (!donors.length) {
    console.log('No donor records found.');
    return;
  }
  console.log('---');
  for (const row of donors) {
    const proj = row.projection_bands && row.projection_bands.mid ? row.projection_bands.mid : { low: 0, mid: 0, high: 0, days: 30 };
    console.log(`${row.donor_id}: flops=${Number(row.donor_flops || 0).toFixed(3)} units=${Number(row.accepted_work_units || 0)} accrued=${Number(row.accrued_rewards || 0).toFixed(6)} settled=${Number(row.settled_rewards || 0).toFixed(6)} proj${Number(proj.days || 0)}d=[${Number(proj.low || 0).toFixed(3)}, ${Number(proj.mid || 0).toFixed(3)}, ${Number(proj.high || 0).toFixed(3)}]`);
  }
}

function cmdDashboard(args: AnyObj) {
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  const strict = toBool(args.strict, false);
  const human = toBool(args.human, false) || cleanText(args.format, 40).toLowerCase() === 'human';

  const out = {
    ...buildDashboard(policy, args),
    policy_path: rel(policy.policy_path),
    latest_path: rel(policy.paths.latest_path)
  };

  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.history_path, {
    ts: out.ts,
    type: out.type,
    ok: out.ok,
    dashboard_receipt_id: out.dashboard_receipt_id,
    donor_count: out.donor_count,
    totals: out.totals
  });

  if (human) {
    printHuman(out);
    process.exit(out.ok || !strict ? 0 : 1);
    return;
  }

  emit(out, out.ok || !strict ? 0 : 1);
}

function cmdRollback(args: AnyObj) {
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  const reason = cleanText(args.reason || args.note || 'manual_dashboard_rollback', 320);

  const row = {
    ts: nowIso(),
    type: 'donor_mining_dashboard_rollback',
    rollback_id: `dash_rb_${stableHash(`${reason}|${Date.now()}`, 12)}`,
    reason,
    actor: cleanText(args.actor || 'operator', 120) || 'operator'
  };
  appendJsonl(policy.paths.rollback_log_path, row);

  emit({
    ok: true,
    type: 'donor_mining_dashboard_rollback',
    lane_id: 'V3-RACE-199',
    ...row,
    rollback_log_path: rel(policy.paths.rollback_log_path)
  }, 0);
}

function cmdStatus(args: AnyObj) {
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);

  emit({
    ok: true,
    type: 'donor_mining_dashboard_status',
    ts: nowIso(),
    latest: readJson(policy.paths.latest_path, null),
    policy_path: rel(policy.policy_path),
    latest_path: rel(policy.paths.latest_path),
    history_path: rel(policy.paths.history_path)
  }, 0);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'dashboard', 80).toLowerCase();

  if (args.help || ['help', '--help', '-h'].includes(cmd)) {
    usage();
    process.exit(0);
  }

  if (cmd === 'dashboard' || cmd === 'run') return cmdDashboard(args);
  if (cmd === 'rollback') return cmdRollback(args);
  if (cmd === 'status') return cmdStatus(args);

  usage();
  emit({ ok: false, error: `unknown_command:${cmd}` }, 2);
}

main();
