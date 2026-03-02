#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-130
 * Global Directive Fund governance lane.
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

const POLICY_PATH = process.env.GLOBAL_DIRECTIVE_FUND_POLICY_PATH
  ? path.resolve(process.env.GLOBAL_DIRECTIVE_FUND_POLICY_PATH)
  : path.join(ROOT, 'config', 'global_directive_fund_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/economy/global_directive_fund.js allocate --owner=<owner_id> --allocation-pct=<0..1> --objective=<objective_id>');
  console.log('  node systems/economy/global_directive_fund.js vote --owner=<owner_id> --objective=<objective_id> --choice=approve|reject [--weight=<n>]');
  console.log('  node systems/economy/global_directive_fund.js status [--owner=<owner_id>] [--objective=<objective_id>]');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    strict_default: true,
    constraints: {
      min_allocation_pct: 0,
      max_allocation_pct: 0.5
    },
    paths: {
      memory_preferences_dir: 'memory/economy/preferences',
      adaptive_index_path: 'adaptive/economy/preferences/index.json',
      objectives_path: 'state/economy/global_directive_fund/objectives.json',
      votes_path: 'state/economy/global_directive_fund/votes.jsonl',
      latest_path: 'state/economy/global_directive_fund/latest.json',
      receipts_path: 'state/economy/global_directive_fund/receipts.jsonl'
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
      max_allocation_pct: clampNumber(constraints.max_allocation_pct, 0, 1, base.constraints.max_allocation_pct)
    },
    paths: {
      memory_preferences_dir: resolvePath(paths.memory_preferences_dir, base.paths.memory_preferences_dir),
      adaptive_index_path: resolvePath(paths.adaptive_index_path, base.paths.adaptive_index_path),
      objectives_path: resolvePath(paths.objectives_path, base.paths.objectives_path),
      votes_path: resolvePath(paths.votes_path, base.paths.votes_path),
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
  return readJson(prefFile(policy, ownerId), {
    owner_id: ownerId,
    allocation_pct: 0,
    objective_id: 'global_objective_default',
    updated_at: null
  });
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
  const next = (index.preferences || []).filter((entry: any) => String(entry.owner_id) !== String(row.owner_id));
  next.push(row);
  index.preferences = next.sort((a: any, b: any) => String(a.owner_id).localeCompare(String(b.owner_id)));
}

function loadObjectives(policy: any) {
  const row = readJson(policy.paths.objectives_path, { objectives: [] });
  return {
    objectives: Array.isArray(row && row.objectives) ? row.objectives : []
  };
}

function saveObjectives(policy: any, payload: any) {
  writeJsonAtomic(policy.paths.objectives_path, payload);
}

function record(policy: any, out: any) {
  appendJsonl(policy.paths.receipts_path, out);
  writeJsonAtomic(policy.paths.latest_path, out);
}

function allocate(policy: any, args: any) {
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
  const pref = loadPref(policy, ownerId);
  const nextPref = {
    ...pref,
    owner_id: ownerId,
    objective_id: objectiveId,
    allocation_pct: allocationPct,
    updated_at: now
  };
  savePref(policy, nextPref);

  const adaptive = loadAdaptiveIndex(policy);
  upsertAdaptive(adaptive, {
    owner_id: ownerId,
    objective_id: objectiveId,
    allocation_pct: allocationPct,
    updated_at: now
  });
  saveAdaptiveIndex(policy, adaptive);

  const objectives = loadObjectives(policy);
  const rest = objectives.objectives.filter((row: any) => String(row.objective_id) !== objectiveId);
  rest.push({
    objective_id: objectiveId,
    updated_at: now,
    allocated_owner_count: (rest.find((row: any) => String(row.objective_id) === objectiveId) || {}).allocated_owner_count || 0
  });
  objectives.objectives = rest.sort((a: any, b: any) => String(a.objective_id).localeCompare(String(b.objective_id)));
  saveObjectives(policy, objectives);

  return {
    ok: true,
    action: 'allocate',
    lane_id: 'V3-RACE-130',
    ts: now,
    owner_id: ownerId,
    objective_id: objectiveId,
    allocation_pct: allocationPct,
    artifacts: {
      memory_preference_path: rel(prefFile(policy, ownerId)),
      adaptive_index_path: rel(policy.paths.adaptive_index_path),
      objectives_path: rel(policy.paths.objectives_path),
      policy_path: rel(policy.policy_path)
    }
  };
}

function vote(policy: any, args: any) {
  const ownerId = normalizeToken(args.owner || args.owner_id, 120);
  const objectiveId = normalizeToken(args.objective || args.objective_id, 120);
  const choice = normalizeToken(args.choice || 'approve', 32) || 'approve';
  const weight = clampNumber(args.weight, 0, 1e6, 1);
  if (!ownerId || !objectiveId) return { ok: false, error: 'missing_vote_fields' };
  if (!['approve', 'reject'].includes(choice)) return { ok: false, error: 'invalid_choice', choice };

  const now = nowIso();
  const row = {
    ts: now,
    type: 'global_directive_vote',
    vote_id: `gdf_vote_${stableHash(`${ownerId}|${objectiveId}|${choice}|${Date.now()}`, 18)}`,
    owner_id: ownerId,
    objective_id: objectiveId,
    choice,
    weight
  };
  appendJsonl(policy.paths.votes_path, row);
  return {
    ok: true,
    action: 'vote',
    lane_id: 'V3-RACE-130',
    ...row,
    artifacts: {
      votes_path: rel(policy.paths.votes_path),
      policy_path: rel(policy.policy_path)
    }
  };
}

function status(policy: any, args: any) {
  const ownerId = normalizeToken(args.owner || args.owner_id, 120);
  const objectiveId = normalizeToken(args.objective || args.objective_id, 120);
  const objectives = loadObjectives(policy);
  const votes = String(require('fs').existsSync(policy.paths.votes_path)
    ? require('fs').readFileSync(policy.paths.votes_path, 'utf8')
    : '')
    .split('\n')
    .filter(Boolean)
    .map((line: string) => {
      try { return JSON.parse(line); } catch { return null; }
    })
    .filter(Boolean);

  let pref = null;
  if (ownerId) pref = loadPref(policy, ownerId);
  const filteredVotes = votes.filter((row: any) => {
    if (ownerId && String(row.owner_id) !== ownerId) return false;
    if (objectiveId && String(row.objective_id) !== objectiveId) return false;
    return true;
  });
  const filteredObjectives = objectives.objectives.filter((row: any) => {
    if (objectiveId && String(row.objective_id) !== objectiveId) return false;
    return true;
  });

  return {
    ok: true,
    action: 'status',
    lane_id: 'V3-RACE-130',
    ts: nowIso(),
    owner_id: ownerId || null,
    objective_id: objectiveId || null,
    preference: pref,
    objective_count: filteredObjectives.length,
    vote_count: filteredVotes.length,
    objectives: filteredObjectives.slice(0, 120),
    recent_votes: filteredVotes.slice(-120),
    artifacts: {
      memory_preferences_dir: rel(policy.paths.memory_preferences_dir),
      adaptive_index_path: rel(policy.paths.adaptive_index_path),
      objectives_path: rel(policy.paths.objectives_path),
      votes_path: rel(policy.paths.votes_path),
      policy_path: rel(policy.policy_path)
    }
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 80) || 'status';
  if (cmd === 'help' || args.help) {
    usage();
    return emit({ ok: true, type: 'global_directive_fund', action: 'help', ts: nowIso() }, 0);
  }
  const policy = loadPolicy(args.policy ? String(args.policy) : undefined);
  if (policy.enabled !== true) {
    return emit({ ok: false, type: 'global_directive_fund', action: cmd, error: 'policy_disabled', ts: nowIso() }, 2);
  }

  let out;
  if (cmd === 'allocate') out = allocate(policy, args);
  else if (cmd === 'vote') out = vote(policy, args);
  else if (cmd === 'status') out = status(policy, args);
  else {
    usage();
    return emit({ ok: false, type: 'global_directive_fund', action: cmd, error: 'unknown_command', ts: nowIso() }, 2);
  }

  const payload = {
    ...out,
    type: 'global_directive_fund',
    policy_version: policy.version
  };
  record(policy, payload);
  const strict = toBool(args.strict, policy.strict_default);
  return emit(payload, payload.ok || !strict ? 0 : 2);
}

main();
