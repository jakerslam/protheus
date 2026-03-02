#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-130
 * Sovereign Token + Global Directive Fund Layer (token engine component).
 *
 * User-specific preferences:
 *   - memory/economy/preferences/
 *   - adaptive/economy/preferences/
 *
 * Permanent policy/runtime logic:
 *   - systems/economy/
 *   - config/
 */

const path = require('path');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  clampNumber,
  toBool,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

const POLICY_PATH = process.env.PROTHEUS_TOKEN_ENGINE_POLICY_PATH
  ? path.resolve(process.env.PROTHEUS_TOKEN_ENGINE_POLICY_PATH)
  : path.join(ROOT, 'config', 'protheus_token_engine_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/economy/protheus_token_engine.js configure --owner=<owner_id> --allocation-pct=<0..1> --objective=<id>');
  console.log('  node systems/economy/protheus_token_engine.js mint --owner=<owner_id> --amount=<tokens> [--reason=<name>]');
  console.log('  node systems/economy/protheus_token_engine.js transfer --from=<owner_id> --to=<owner_id> --amount=<tokens> [--reason=<name>]');
  console.log('  node systems/economy/protheus_token_engine.js status [--owner=<owner_id>]');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    strict_default: true,
    constraints: {
      min_allocation_pct: 0,
      max_allocation_pct: 0.5,
      max_tx_amount: 1000000
    },
    paths: {
      memory_preferences_dir: 'memory/economy/preferences',
      adaptive_index_path: 'adaptive/economy/preferences/index.json',
      balances_path: 'state/economy/protheus_token_balances.json',
      ledger_path: 'state/economy/protheus_token_ledger.jsonl',
      bridge_receipts_path: 'state/blockchain/protheus_token_bridge.jsonl',
      latest_path: 'state/economy/protheus_token_engine/latest.json',
      receipts_path: 'state/economy/protheus_token_engine/receipts.jsonl'
    }
  };
}

function loadPolicy(policyPath = POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const constraints = raw.constraints && typeof raw.constraints === 'object' ? raw.constraints : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 32) || base.version,
    enabled: raw.enabled !== false,
    strict_default: toBool(raw.strict_default, base.strict_default),
    constraints: {
      min_allocation_pct: clampNumber(constraints.min_allocation_pct, 0, 1, base.constraints.min_allocation_pct),
      max_allocation_pct: clampNumber(constraints.max_allocation_pct, 0, 1, base.constraints.max_allocation_pct),
      max_tx_amount: clampNumber(constraints.max_tx_amount, 1, 1e12, base.constraints.max_tx_amount)
    },
    paths: {
      memory_preferences_dir: resolvePath(paths.memory_preferences_dir, base.paths.memory_preferences_dir),
      adaptive_index_path: resolvePath(paths.adaptive_index_path, base.paths.adaptive_index_path),
      balances_path: resolvePath(paths.balances_path, base.paths.balances_path),
      ledger_path: resolvePath(paths.ledger_path, base.paths.ledger_path),
      bridge_receipts_path: resolvePath(paths.bridge_receipts_path, base.paths.bridge_receipts_path),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function rel(absPath: string) {
  return path.relative(ROOT, absPath).replace(/\\/g, '/');
}

function prefFile(policy: any, ownerId: string) {
  return path.join(policy.paths.memory_preferences_dir, `${ownerId}.json`);
}

function loadPref(policy: any, ownerId: string) {
  const row = readJson(prefFile(policy, ownerId), {
    owner_id: ownerId,
    allocation_pct: 0,
    objective_id: 'global_objective_default',
    updated_at: null
  });
  return {
    owner_id: ownerId,
    allocation_pct: clampNumber(row.allocation_pct, 0, 1, 0),
    objective_id: cleanText(row.objective_id || 'global_objective_default', 120) || 'global_objective_default',
    updated_at: cleanText(row.updated_at || '', 80) || null
  };
}

function savePref(policy: any, pref: any) {
  writeJsonAtomic(prefFile(policy, pref.owner_id), pref);
}

function loadAdaptiveIndex(policy: any) {
  const row = readJson(policy.paths.adaptive_index_path, { preferences: [] });
  return {
    preferences: Array.isArray(row && row.preferences) ? row.preferences : []
  };
}

function saveAdaptiveIndex(policy: any, index: any) {
  writeJsonAtomic(policy.paths.adaptive_index_path, index);
}

function upsertAdaptive(index: any, row: any) {
  const rest = (index.preferences || []).filter((entry: any) => String(entry.owner_id) !== String(row.owner_id));
  rest.push(row);
  index.preferences = rest.sort((a: any, b: any) => String(a.owner_id).localeCompare(String(b.owner_id)));
}

function loadBalances(policy: any) {
  const row = readJson(policy.paths.balances_path, { balances: {} });
  const balances = row && row.balances && typeof row.balances === 'object' ? row.balances : {};
  return { balances };
}

function saveBalances(policy: any, payload: any) {
  writeJsonAtomic(policy.paths.balances_path, payload);
}

function record(policy: any, out: any) {
  appendJsonl(policy.paths.receipts_path, out);
  writeJsonAtomic(policy.paths.latest_path, out);
}

function setPref(policy: any, args: any) {
  const ownerId = normalizeToken(args.owner || args.owner_id, 120);
  const objectiveId = normalizeToken(args.objective || args.objective_id, 120) || 'global_objective_default';
  const allocationPct = clampNumber(args['allocation-pct'] || args.allocation_pct, 0, 1, 0);
  if (!ownerId) return { ok: false, error: 'missing_owner' };
  if (allocationPct < policy.constraints.min_allocation_pct || allocationPct > policy.constraints.max_allocation_pct) {
    return {
      ok: false,
      error: 'allocation_out_of_bounds',
      min_allocation_pct: policy.constraints.min_allocation_pct,
      max_allocation_pct: policy.constraints.max_allocation_pct,
      allocation_pct: allocationPct
    };
  }
  const now = nowIso();
  const pref = {
    owner_id: ownerId,
    allocation_pct: allocationPct,
    objective_id: objectiveId,
    updated_at: now
  };
  savePref(policy, pref);

  const adaptive = loadAdaptiveIndex(policy);
  upsertAdaptive(adaptive, {
    owner_id: ownerId,
    allocation_pct: allocationPct,
    objective_id: objectiveId,
    updated_at: now
  });
  saveAdaptiveIndex(policy, adaptive);

  return {
    ok: true,
    action: 'configure',
    lane_id: 'V3-RACE-130',
    ts: now,
    owner_id: ownerId,
    allocation_pct: allocationPct,
    objective_id: objectiveId,
    artifacts: {
      memory_preference_path: rel(prefFile(policy, ownerId)),
      adaptive_index_path: rel(policy.paths.adaptive_index_path),
      policy_path: rel(policy.policy_path)
    }
  };
}

function mint(policy: any, args: any) {
  const ownerId = normalizeToken(args.owner || args.owner_id, 120);
  const amount = clampNumber(args.amount, 0, policy.constraints.max_tx_amount, 0);
  const reason = normalizeToken(args.reason || 'mint', 120) || 'mint';
  if (!ownerId || amount <= 0) return { ok: false, error: 'missing_owner_or_amount' };

  const state = loadBalances(policy);
  const cur = clampNumber(state.balances[ownerId], -1e18, 1e18, 0);
  const next = Number((cur + amount).toFixed(6));
  state.balances[ownerId] = next;
  saveBalances(policy, state);

  const row = {
    ts: nowIso(),
    type: 'protheus_token_mint',
    owner_id: ownerId,
    amount,
    balance_after: next,
    reason,
    tx_id: `ptx_${stableHash(`${ownerId}|${amount}|${reason}|${Date.now()}`, 18)}`
  };
  appendJsonl(policy.paths.ledger_path, row);
  appendJsonl(policy.paths.bridge_receipts_path, {
    ts: row.ts,
    type: 'protheus_token_bridge_hint',
    tx_id: row.tx_id,
    owner_id: ownerId,
    amount,
    reason,
    bridge_contract: 'V3-BLK-001'
  });

  return {
    ok: true,
    action: 'mint',
    lane_id: 'V3-RACE-130',
    ...row,
    artifacts: {
      balances_path: rel(policy.paths.balances_path),
      ledger_path: rel(policy.paths.ledger_path),
      bridge_receipts_path: rel(policy.paths.bridge_receipts_path),
      policy_path: rel(policy.policy_path)
    }
  };
}

function transfer(policy: any, args: any) {
  const from = normalizeToken(args.from, 120);
  const to = normalizeToken(args.to, 120);
  const amount = clampNumber(args.amount, 0, policy.constraints.max_tx_amount, 0);
  const reason = normalizeToken(args.reason || 'transfer', 120) || 'transfer';
  if (!from || !to || amount <= 0) return { ok: false, error: 'missing_transfer_fields' };
  if (from === to) return { ok: false, error: 'same_source_target' };

  const state = loadBalances(policy);
  const fromBal = clampNumber(state.balances[from], -1e18, 1e18, 0);
  const toBal = clampNumber(state.balances[to], -1e18, 1e18, 0);
  if (fromBal < amount) return { ok: false, error: 'insufficient_balance', from_balance: fromBal, amount };
  const nextFrom = Number((fromBal - amount).toFixed(6));
  const nextTo = Number((toBal + amount).toFixed(6));
  state.balances[from] = nextFrom;
  state.balances[to] = nextTo;
  saveBalances(policy, state);

  const row = {
    ts: nowIso(),
    type: 'protheus_token_transfer',
    from_owner_id: from,
    to_owner_id: to,
    amount,
    from_balance_after: nextFrom,
    to_balance_after: nextTo,
    reason,
    tx_id: `ptx_${stableHash(`${from}|${to}|${amount}|${Date.now()}`, 18)}`
  };
  appendJsonl(policy.paths.ledger_path, row);
  return {
    ok: true,
    action: 'transfer',
    lane_id: 'V3-RACE-130',
    ...row,
    artifacts: {
      balances_path: rel(policy.paths.balances_path),
      ledger_path: rel(policy.paths.ledger_path),
      policy_path: rel(policy.policy_path)
    }
  };
}

function status(policy: any, args: any) {
  const ownerId = normalizeToken(args.owner || args.owner_id, 120);
  const state = loadBalances(policy);
  if (ownerId) {
    const pref = loadPref(policy, ownerId);
    return {
      ok: true,
      action: 'status',
      lane_id: 'V3-RACE-130',
      ts: nowIso(),
      owner_id: ownerId,
      balance: clampNumber(state.balances[ownerId], -1e18, 1e18, 0),
      preference: pref,
      artifacts: {
        memory_preference_path: rel(prefFile(policy, ownerId)),
        balances_path: rel(policy.paths.balances_path),
        adaptive_index_path: rel(policy.paths.adaptive_index_path),
        policy_path: rel(policy.policy_path)
      }
    };
  }
  return {
    ok: true,
    action: 'status',
    lane_id: 'V3-RACE-130',
    ts: nowIso(),
    owner_count: Object.keys(state.balances).length,
    balances: state.balances,
    artifacts: {
      balances_path: rel(policy.paths.balances_path),
      adaptive_index_path: rel(policy.paths.adaptive_index_path),
      policy_path: rel(policy.policy_path)
    }
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 80) || 'status';
  if (cmd === 'help' || args.help) {
    usage();
    return emit({ ok: true, type: 'protheus_token_engine', action: 'help', ts: nowIso() }, 0);
  }
  const policy = loadPolicy(args.policy ? String(args.policy) : undefined);
  if (policy.enabled !== true) {
    return emit({ ok: false, type: 'protheus_token_engine', action: cmd, error: 'policy_disabled', ts: nowIso() }, 2);
  }

  let out;
  if (cmd === 'configure') out = setPref(policy, args);
  else if (cmd === 'mint') out = mint(policy, args);
  else if (cmd === 'transfer') out = transfer(policy, args);
  else if (cmd === 'status') out = status(policy, args);
  else {
    usage();
    return emit({ ok: false, type: 'protheus_token_engine', action: cmd, error: 'unknown_command', ts: nowIso() }, 2);
  }

  const payload = {
    ...out,
    type: 'protheus_token_engine',
    policy_version: policy.version
  };
  record(policy, payload);
  const strict = toBool(args.strict, policy.strict_default);
  return emit(payload, payload.ok || !strict ? 0 : 2);
}

main();
