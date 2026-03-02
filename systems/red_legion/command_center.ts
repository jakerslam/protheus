#!/usr/bin/env node
'use strict';
export {};

/**
 * systems/red_legion/command_center.js
 *
 * Operational command surface for Red Legion roster + mission coordination.
 */

const path = require('path');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  clampInt,
  toBool,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  relPath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

const POLICY_PATH = process.env.RED_LEGION_COMMAND_CENTER_POLICY_PATH
  ? path.resolve(process.env.RED_LEGION_COMMAND_CENTER_POLICY_PATH)
  : path.join(ROOT, 'config/red_legion_command_center_policy.json');

const DEFAULT_POLICY = {
  version: '1.0',
  enabled: true,
  strict_default: true,
  rank_order: ['recruit', 'operator', 'sentinel', 'captain', 'warden'],
  max_open_missions_per_operator: 3,
  paths: {
    state_path: 'state/red_legion/command_center/state.json',
    latest_path: 'state/red_legion/command_center/latest.json',
    receipts_path: 'state/red_legion/command_center/receipts.jsonl',
    history_path: 'state/red_legion/command_center/history.jsonl'
  }
};

function normalizePolicy(policyPath) {
  const raw = readJson(policyPath, {});
  const src = raw && typeof raw === 'object' ? raw : {};
  const pathsRaw = src.paths && typeof src.paths === 'object' ? src.paths : {};
  const rankOrder = Array.isArray(src.rank_order)
    ? src.rank_order.map((row) => normalizeToken(row, 32)).filter(Boolean)
    : DEFAULT_POLICY.rank_order;
  return {
    version: cleanText(src.version || DEFAULT_POLICY.version, 32) || DEFAULT_POLICY.version,
    enabled: src.enabled !== false,
    strict_default: toBool(src.strict_default, DEFAULT_POLICY.strict_default),
    rank_order: rankOrder.length > 0 ? rankOrder : DEFAULT_POLICY.rank_order,
    max_open_missions_per_operator: clampInt(src.max_open_missions_per_operator, 1, 20, DEFAULT_POLICY.max_open_missions_per_operator),
    paths: {
      state_path: resolvePath(pathsRaw.state_path, DEFAULT_POLICY.paths.state_path),
      latest_path: resolvePath(pathsRaw.latest_path, DEFAULT_POLICY.paths.latest_path),
      receipts_path: resolvePath(pathsRaw.receipts_path, DEFAULT_POLICY.paths.receipts_path),
      history_path: resolvePath(pathsRaw.history_path, DEFAULT_POLICY.paths.history_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function loadState(policy) {
  const raw = readJson(policy.paths.state_path, {});
  return {
    schema_id: 'red_legion_command_center_state_v1',
    schema_version: '1.0',
    run_count: Math.max(0, Number(raw && raw.run_count || 0)),
    last_action: raw && raw.last_action ? cleanText(raw.last_action, 80) : null,
    last_ok: typeof (raw && raw.last_ok) === 'boolean' ? raw.last_ok : null,
    last_ts: raw && raw.last_ts ? cleanText(raw.last_ts, 80) : null,
    roster: raw && raw.roster && typeof raw.roster === 'object' ? raw.roster : {},
    missions: raw && raw.missions && typeof raw.missions === 'object' ? raw.missions : {}
  };
}

function persist(policy, out, state, apply) {
  if (!apply) return;
  writeJsonAtomic(policy.paths.state_path, state);
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  appendJsonl(policy.paths.history_path, {
    ts: out.ts,
    action: out.action,
    ok: out.ok,
    operator_id: out.operator_id || null,
    mission_id: out.mission && out.mission.mission_id || null
  });
}

function rankIndex(policy, rank) {
  return policy.rank_order.indexOf(normalizeToken(rank, 32));
}

function baseOut(policy, state, action, args, extra = {}) {
  const strict = toBool(args.strict, policy.strict_default);
  const apply = toBool(args.apply, true);
  const ok = extra.ok !== false;
  const nextState = {
    ...state,
    run_count: state.run_count + 1,
    last_action: action,
    last_ok: ok,
    last_ts: nowIso()
  };
  const out = {
    ok,
    type: 'red_legion_command_center',
    action,
    ts: nowIso(),
    strict,
    apply,
    policy_version: policy.version,
    policy_path: relPath(policy.policy_path),
    state: nextState,
    ...extra
  };
  return { out, nextState, strict, apply, ok };
}

function openMissionCountFor(state, operatorId) {
  return Object.values(state.missions || {}).filter((row) => row && row.operator_id === operatorId && row.status === 'open').length;
}

function cmdEnlist(policy, args) {
  const state = loadState(policy);
  const operatorId = cleanText(args['operator-id'] || args.operator_id || args.id || '', 120);
  if (!operatorId) {
    emit({ ok: false, type: 'red_legion_command_center', error: 'operator_id_required' }, 2);
  }
  const alias = cleanText(args.alias || operatorId, 120) || operatorId;
  const rank = normalizeToken(args.rank || 'recruit', 32) || 'recruit';
  if (rankIndex(policy, rank) < 0) {
    emit({ ok: false, type: 'red_legion_command_center', error: 'invalid_rank', rank, allowed_ranks: policy.rank_order }, 2);
  }

  const rosterRow = {
    operator_id: operatorId,
    alias,
    rank,
    enlisted_at: nowIso(),
    updated_at: nowIso(),
    status: 'active'
  };

  const { out, nextState, strict, apply, ok } = baseOut(policy, state, 'enlist', args, {
    operator_id: operatorId,
    operator: rosterRow
  });

  if (ok && apply) {
    nextState.roster[operatorId] = rosterRow;
  }

  persist(policy, out, nextState, apply);
  emit(out, ok || !strict ? 0 : 2);
}

function cmdPromote(policy, args) {
  const state = loadState(policy);
  const operatorId = cleanText(args['operator-id'] || args.operator_id || args.id || '', 120);
  const targetRank = normalizeToken(args.rank || '', 32);
  const current = operatorId ? (state.roster[operatorId] || null) : null;
  if (!current) emit({ ok: false, type: 'red_legion_command_center', error: 'operator_not_found', operator_id: operatorId }, 2);
  if (!targetRank || rankIndex(policy, targetRank) < 0) {
    emit({ ok: false, type: 'red_legion_command_center', error: 'invalid_rank', rank: targetRank, allowed_ranks: policy.rank_order }, 2);
  }

  const curIdx = rankIndex(policy, current.rank);
  const nextIdx = rankIndex(policy, targetRank);
  if (nextIdx < curIdx) {
    emit({ ok: false, type: 'red_legion_command_center', error: 'rank_regression_blocked', current_rank: current.rank, target_rank: targetRank }, 2);
  }

  const updated = {
    ...current,
    rank: targetRank,
    promoted_at: nowIso(),
    updated_at: nowIso()
  };

  const { out, nextState, strict, apply, ok } = baseOut(policy, state, 'promote', args, {
    operator_id: operatorId,
    from_rank: current.rank,
    to_rank: targetRank,
    operator: updated
  });

  if (ok && apply) {
    nextState.roster[operatorId] = updated;
  }

  persist(policy, out, nextState, apply);
  emit(out, ok || !strict ? 0 : 2);
}

function cmdMission(policy, args) {
  const state = loadState(policy);
  const operatorId = cleanText(args['operator-id'] || args.operator_id || args.id || '', 120);
  const operator = operatorId ? (state.roster[operatorId] || null) : null;
  if (!operator) emit({ ok: false, type: 'red_legion_command_center', error: 'operator_not_found', operator_id: operatorId }, 2);

  const objective = cleanText(args.objective || args.goal || 'unspecified_objective', 240) || 'unspecified_objective';
  const riskTier = clampInt(args['risk-tier'] || args.risk_tier, 1, 4, 2);
  const openCount = openMissionCountFor(state, operatorId);
  if (openCount >= policy.max_open_missions_per_operator) {
    emit({
      ok: false,
      type: 'red_legion_command_center',
      error: 'operator_open_mission_cap_reached',
      operator_id: operatorId,
      open_missions: openCount,
      cap: policy.max_open_missions_per_operator
    }, 2);
  }

  const mission = {
    mission_id: `rlm_${stableHash(`${operatorId}|${objective}|${Date.now()}`, 16)}`,
    operator_id: operatorId,
    objective,
    risk_tier: riskTier,
    created_at: nowIso(),
    status: 'open'
  };

  const { out, nextState, strict, apply, ok } = baseOut(policy, state, 'mission', args, {
    operator_id: operatorId,
    mission
  });

  if (ok && apply) {
    nextState.missions[mission.mission_id] = mission;
  }

  persist(policy, out, nextState, apply);
  emit(out, ok || !strict ? 0 : 2);
}

function cmdStatus(policy, args) {
  const state = loadState(policy);
  const operatorId = cleanText(args['operator-id'] || args.operator_id || args.id || '', 120);
  if (operatorId) {
    const operator = state.roster[operatorId] || null;
    const missions = Object.values(state.missions || {}).filter((row) => row && row.operator_id === operatorId);
    emit({
      ok: !!operator,
      type: 'red_legion_command_center',
      action: 'status',
      ts: nowIso(),
      operator_id: operatorId,
      operator,
      mission_count: missions.length,
      open_mission_count: missions.filter((row) => row.status === 'open').length,
      missions
    }, operator ? 0 : 2);
  }

  emit({
    ok: true,
    type: 'red_legion_command_center',
    action: 'status',
    ts: nowIso(),
    operator_count: Object.keys(state.roster).length,
    mission_count: Object.keys(state.missions).length,
    open_mission_count: Object.values(state.missions).filter((row) => row && row.status === 'open').length,
    state,
    policy_path: relPath(policy.policy_path)
  }, 0);
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/red_legion/command_center.js enlist --operator-id=<id> [--alias=<name>] [--rank=recruit]');
  console.log('  node systems/red_legion/command_center.js promote --operator-id=<id> --rank=<rank>');
  console.log('  node systems/red_legion/command_center.js mission --operator-id=<id> --objective=<text> [--risk-tier=1..4]');
  console.log('  node systems/red_legion/command_center.js status [--operator-id=<id>]');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const action = normalizeToken(args._[0] || 'status', 80) || 'status';
  if (args.help || action === 'help') {
    usage();
    emit({ ok: true, type: 'red_legion_command_center', action: 'help', ts: nowIso() }, 0);
  }

  const policy = normalizePolicy(args.policy ? String(args.policy) : POLICY_PATH);
  if (policy.enabled !== true) emit({ ok: false, type: 'red_legion_command_center', error: 'lane_disabled', policy_path: relPath(policy.policy_path) }, 2);

  if (action === 'status') return cmdStatus(policy, args);
  if (action === 'enlist') return cmdEnlist(policy, args);
  if (action === 'promote') return cmdPromote(policy, args);
  if (action === 'mission') return cmdMission(policy, args);

  usage();
  emit({ ok: false, type: 'red_legion_command_center', error: 'unknown_action', action }, 2);
}

if (require.main === module) {
  main();
}
