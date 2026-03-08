#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-131
 * Seed spawn lineage/inheritance helper.
 *
 * User-specific lineage preferences:
 *   - memory/lineage/
 * Adaptive heuristics:
 *   - adaptive/lineage/
 * Permanent runtime logic/policy:
 *   - systems/spawn/
 *   - config/
 */

const path = require('path');
const { spawnSync } = require('child_process');
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

const POLICY_PATH = process.env.SEED_SPAWN_LINEAGE_POLICY_PATH
  ? path.resolve(process.env.SEED_SPAWN_LINEAGE_POLICY_PATH)
  : path.join(ROOT, 'config', 'seed_spawn_lineage_policy.json');

const EVENT_STREAM_CMD = process.env.EVENT_STREAM_CONTROL_PLANE_CMD
  ? path.resolve(process.env.EVENT_STREAM_CONTROL_PLANE_CMD)
  : path.join(ROOT, 'systems', 'ops', 'event_sourced_control_plane.js');

function usage() {
  console.log('Usage:');
  console.log('  node systems/spawn/seed_spawn_lineage.js configure --owner=<owner_id> [--directives=a,b] [--badges=a,b] [--contracts=a,b] [--parent-route-tithe-pct=0.02] [--policy=<path>]');
  console.log('  node systems/spawn/seed_spawn_lineage.js preview --owner=<owner_id> --parent=<parent_id> --child=<child_id> [--profile=seed_spawn] [--apply=1] [--policy=<path>]');
  console.log('  node systems/spawn/seed_spawn_lineage.js status [--owner=<owner_id>]');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    strict_default: true,
    inheritance: {
      enabled_profiles: ['seed_spawn'],
      max_directives: 16,
      max_badges: 32,
      max_contract_refs: 32,
      max_parent_route_tithe_pct: 0.15,
      allow_parent_route_tithe: true
    },
    event_stream: {
      enabled: true,
      publish: true,
      stream: 'spawn.lineage'
    },
    paths: {
      memory_dir: 'memory/lineage',
      adaptive_index_path: 'adaptive/lineage/seed_spawn_index.json',
      contracts_dir: 'state/spawn/seed_spawn_lineage/contracts',
      latest_path: 'state/spawn/seed_spawn_lineage/latest.json',
      history_path: 'state/spawn/seed_spawn_lineage/history.jsonl',
      receipts_path: 'state/spawn/seed_spawn_lineage/receipts.jsonl'
    }
  };
}

function loadPolicy(policyPath = POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const inheritance = raw.inheritance && typeof raw.inheritance === 'object' ? raw.inheritance : {};
  const eventStream = raw.event_stream && typeof raw.event_stream === 'object' ? raw.event_stream : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  const profiles = Array.isArray(inheritance.enabled_profiles) ? inheritance.enabled_profiles : base.inheritance.enabled_profiles;
  return {
    version: cleanText(raw.version || base.version, 32) || base.version,
    enabled: raw.enabled !== false,
    strict_default: toBool(raw.strict_default, base.strict_default),
    inheritance: {
      enabled_profiles: profiles.map((row: unknown) => normalizeToken(row, 80)).filter(Boolean),
      max_directives: Math.max(1, Math.min(128, Number(inheritance.max_directives || base.inheritance.max_directives))),
      max_badges: Math.max(1, Math.min(256, Number(inheritance.max_badges || base.inheritance.max_badges))),
      max_contract_refs: Math.max(1, Math.min(256, Number(inheritance.max_contract_refs || base.inheritance.max_contract_refs))),
      max_parent_route_tithe_pct: clampNumber(
        inheritance.max_parent_route_tithe_pct,
        0,
        0.9,
        base.inheritance.max_parent_route_tithe_pct
      ),
      allow_parent_route_tithe: toBool(inheritance.allow_parent_route_tithe, base.inheritance.allow_parent_route_tithe)
    },
    event_stream: {
      enabled: toBool(eventStream.enabled, base.event_stream.enabled),
      publish: toBool(eventStream.publish, base.event_stream.publish),
      stream: normalizeToken(eventStream.stream || base.event_stream.stream, 120) || base.event_stream.stream
    },
    paths: {
      memory_dir: resolvePath(paths.memory_dir, base.paths.memory_dir),
      adaptive_index_path: resolvePath(paths.adaptive_index_path, base.paths.adaptive_index_path),
      contracts_dir: resolvePath(paths.contracts_dir, base.paths.contracts_dir),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function rel(absPath: string) {
  return path.relative(ROOT, absPath).replace(/\\/g, '/');
}

function splitCsv(raw: unknown, max = 64) {
  const txt = cleanText(raw || '', 5000);
  if (!txt) return [];
  return txt
    .split(',')
    .map((row) => normalizeToken(row, 120))
    .filter(Boolean)
    .slice(0, max);
}

function ownerPath(policy: any, ownerId: string) {
  return path.join(policy.paths.memory_dir, `${ownerId}.json`);
}

function loadOwnerConfig(policy: any, ownerId: string) {
  const fallback = {
    owner_id: ownerId,
    directives: [],
    badges: [],
    contract_refs: [],
    parent_route_tithe_pct: 0,
    updated_at: null
  };
  const row = readJson(ownerPath(policy, ownerId), fallback);
  return {
    owner_id: ownerId,
    directives: Array.isArray(row && row.directives) ? row.directives : [],
    badges: Array.isArray(row && row.badges) ? row.badges : [],
    contract_refs: Array.isArray(row && row.contract_refs) ? row.contract_refs : [],
    parent_route_tithe_pct: clampNumber(row && row.parent_route_tithe_pct, 0, 1, 0),
    updated_at: row && row.updated_at ? String(row.updated_at) : null
  };
}

function saveOwnerConfig(policy: any, row: any) {
  writeJsonAtomic(ownerPath(policy, row.owner_id), row);
}

function loadAdaptiveIndex(policy: any) {
  const row = readJson(policy.paths.adaptive_index_path, { owners: [], profiles: {} });
  return {
    owners: Array.isArray(row && row.owners) ? row.owners : [],
    profiles: row && row.profiles && typeof row.profiles === 'object' ? row.profiles : {}
  };
}

function saveAdaptiveIndex(policy: any, row: any) {
  writeJsonAtomic(policy.paths.adaptive_index_path, row);
}

function writeLatestAndReceipt(policy: any, out: any) {
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.history_path, {
    ts: out.ts,
    action: out.action,
    ok: out.ok,
    owner_id: out.owner_id || null,
    child_id: out.child_id || null,
    lineage_contract_id: out.lineage_contract && out.lineage_contract.lineage_contract_id
      ? out.lineage_contract.lineage_contract_id
      : null
  });
  appendJsonl(policy.paths.receipts_path, out);
}

function publishEvent(policy: any, eventName: string, payload: any) {
  if (!policy.event_stream.enabled || !policy.event_stream.publish) {
    return { attempted: false, reason: 'event_stream_disabled' };
  }
  const payloadJson = JSON.stringify({
    lane_id: 'V3-RACE-131',
    ts: nowIso(),
    ...payload
  });
  const proc = spawnSync('node', [
    EVENT_STREAM_CMD,
    'append',
    `--stream=${policy.event_stream.stream}`,
    `--event=${normalizeToken(eventName, 120) || 'seed_spawn_event'}`,
    `--payload_json=${payloadJson}`
  ], {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: 10000
  });
  return {
    attempted: true,
    ok: Number(proc.status || 0) === 0,
    status: Number(proc.status || 0),
    stderr: cleanText(proc.stderr || '', 400),
    stdout: cleanText(proc.stdout || '', 400)
  };
}

function boundedUnique(values: any[], maxItems: number) {
  const out: string[] = [];
  for (const value of values || []) {
    const tok = normalizeToken(value, 120);
    if (!tok) continue;
    if (out.includes(tok)) continue;
    out.push(tok);
    if (out.length >= maxItems) break;
  }
  return out;
}

function buildLineageContract(policy: any, args: any) {
  const ownerId = normalizeToken(args.owner || args.owner_id, 120);
  const parentId = normalizeToken(args.parent || args.parent_id, 120);
  const childId = normalizeToken(args.child || args.child_id, 120);
  const profile = normalizeToken(args.profile || 'seed_spawn', 80) || 'seed_spawn';
  if (!ownerId || !parentId || !childId) {
    return { ok: false, error: 'missing_owner_parent_child' };
  }
  if (!policy.inheritance.enabled_profiles.includes(profile)) {
    return { ok: false, error: 'profile_not_enabled', profile };
  }

  const ownerCfg = loadOwnerConfig(policy, ownerId);
  const directives = boundedUnique(
    ownerCfg.directives.concat(splitCsv(args.directives)),
    policy.inheritance.max_directives
  );
  const badges = boundedUnique(
    ownerCfg.badges.concat(splitCsv(args.badges)),
    policy.inheritance.max_badges
  );
  const contractRefs = boundedUnique(
    ownerCfg.contract_refs.concat(splitCsv(args.contracts || args.contract_refs)),
    policy.inheritance.max_contract_refs
  );

  const requestedTithe = clampNumber(
    args['parent-route-tithe-pct'] != null ? args['parent-route-tithe-pct'] : args.parent_route_tithe_pct,
    0,
    1,
    ownerCfg.parent_route_tithe_pct || 0
  );
  const parentRouteTithePct = policy.inheritance.allow_parent_route_tithe
    ? Math.min(policy.inheritance.max_parent_route_tithe_pct, requestedTithe)
    : 0;

  const ts = nowIso();
  return {
    ok: true,
    owner_id: ownerId,
    parent_id: parentId,
    child_id: childId,
    profile,
    lineage_contract: {
      lineage_contract_id: `lin_${stableHash(`${ownerId}|${parentId}|${childId}|${ts}`, 20)}`,
      profile,
      owner_id: ownerId,
      parent_id: parentId,
      child_id: childId,
      inherited_directives: directives,
      inherited_badges: badges,
      inherited_contract_refs: contractRefs,
      parent_route_tithe_pct: parentRouteTithePct,
      created_at: ts
    }
  };
}

function cmdConfigure(policy: any, args: any) {
  const ownerId = normalizeToken(args.owner || args.owner_id, 120);
  if (!ownerId) {
    return { ok: false, error: 'missing_owner' };
  }
  const directives = boundedUnique(splitCsv(args.directives), policy.inheritance.max_directives);
  const badges = boundedUnique(splitCsv(args.badges), policy.inheritance.max_badges);
  const contractRefs = boundedUnique(splitCsv(args.contracts || args.contract_refs), policy.inheritance.max_contract_refs);
  const parentRouteTithePct = policy.inheritance.allow_parent_route_tithe
    ? clampNumber(
        args['parent-route-tithe-pct'] != null ? args['parent-route-tithe-pct'] : args.parent_route_tithe_pct,
        0,
        policy.inheritance.max_parent_route_tithe_pct,
        0
      )
    : 0;
  const ts = nowIso();
  const row = {
    owner_id: ownerId,
    directives,
    badges,
    contract_refs: contractRefs,
    parent_route_tithe_pct: parentRouteTithePct,
    updated_at: ts
  };
  saveOwnerConfig(policy, row);

  const adaptive = loadAdaptiveIndex(policy);
  const nextOwners = (adaptive.owners || []).filter((item: any) => String(item.owner_id) !== ownerId);
  nextOwners.push({
    owner_id: ownerId,
    directives_count: directives.length,
    badges_count: badges.length,
    contracts_count: contractRefs.length,
    parent_route_tithe_pct: parentRouteTithePct,
    updated_at: ts
  });
  adaptive.owners = nextOwners.sort((a: any, b: any) => String(a.owner_id).localeCompare(String(b.owner_id)));
  saveAdaptiveIndex(policy, adaptive);

  return {
    ok: true,
    action: 'configure',
    lane_id: 'V3-RACE-131',
    ts,
    owner_id: ownerId,
    directives_count: directives.length,
    badges_count: badges.length,
    contracts_count: contractRefs.length,
    parent_route_tithe_pct: parentRouteTithePct,
    artifacts: {
      memory_owner_path: rel(ownerPath(policy, ownerId)),
      adaptive_index_path: rel(policy.paths.adaptive_index_path),
      policy_path: rel(policy.policy_path)
    }
  };
}

function cmdPreview(policy: any, args: any) {
  const apply = toBool(args.apply, false);
  const built = buildLineageContract(policy, args);
  if (!built.ok) return built;

  const ts = nowIso();
  let contractPath = null;
  if (apply) {
    contractPath = path.join(policy.paths.contracts_dir, `${built.child_id}.json`);
    writeJsonAtomic(contractPath, built.lineage_contract);
    const adaptive = loadAdaptiveIndex(policy);
    const currentProfile = adaptive.profiles[built.profile] && typeof adaptive.profiles[built.profile] === 'object'
      ? adaptive.profiles[built.profile]
      : { spawned: 0 };
    adaptive.profiles[built.profile] = {
      spawned: Math.max(0, Number(currentProfile.spawned || 0)) + 1,
      last_child_id: built.child_id,
      updated_at: ts
    };
    saveAdaptiveIndex(policy, adaptive);
  }

  const out = {
    ok: true,
    action: 'preview',
    lane_id: 'V3-RACE-131',
    ts,
    apply,
    owner_id: built.owner_id,
    parent_id: built.parent_id,
    child_id: built.child_id,
    profile: built.profile,
    lineage_contract: built.lineage_contract,
    artifacts: {
      contract_path: contractPath ? rel(contractPath) : null,
      adaptive_index_path: rel(policy.paths.adaptive_index_path),
      receipts_path: rel(policy.paths.receipts_path),
      policy_path: rel(policy.policy_path)
    }
  };

  const stream = publishEvent(policy, 'seed_spawn_contract', {
    owner_id: built.owner_id,
    parent_id: built.parent_id,
    child_id: built.child_id,
    profile: built.profile,
    lineage_contract_id: built.lineage_contract.lineage_contract_id,
    apply
  });
  out.event_stream = stream;
  return out;
}

function cmdStatus(policy: any, args: any) {
  const ownerId = normalizeToken(args.owner || args.owner_id, 120);
  if (ownerId) {
    const ownerCfg = loadOwnerConfig(policy, ownerId);
    return {
      ok: true,
      action: 'status',
      lane_id: 'V3-RACE-131',
      ts: nowIso(),
      owner_id: ownerId,
      owner_config: ownerCfg,
      artifacts: {
        memory_owner_path: rel(ownerPath(policy, ownerId)),
        adaptive_index_path: rel(policy.paths.adaptive_index_path),
        policy_path: rel(policy.policy_path)
      }
    };
  }
  const adaptive = loadAdaptiveIndex(policy);
  return {
    ok: true,
    action: 'status',
    lane_id: 'V3-RACE-131',
    ts: nowIso(),
    owner_count: Array.isArray(adaptive.owners) ? adaptive.owners.length : 0,
    profile_stats: adaptive.profiles,
    artifacts: {
      adaptive_index_path: rel(policy.paths.adaptive_index_path),
      policy_path: rel(policy.policy_path)
    }
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = String(args._[0] || '').trim().toLowerCase();
  if (!cmd || cmd === '--help' || cmd === 'help' || cmd === '-h' || args.help) {
    usage();
    process.exit(0);
  }
  const policy = loadPolicy(args.policy ? String(args.policy) : POLICY_PATH);
  let out;
  if (cmd === 'configure') out = cmdConfigure(policy, args);
  else if (cmd === 'preview' || cmd === 'inherit') out = cmdPreview(policy, args);
  else if (cmd === 'status') out = cmdStatus(policy, args);
  else {
    usage();
    process.exit(2);
    return;
  }
  writeLatestAndReceipt(policy, out);
  emit(out, out.ok ? 0 : 2);
}

if (require.main === module) {
  main();
}

module.exports = {
  loadPolicy,
  buildLineageContract
};
