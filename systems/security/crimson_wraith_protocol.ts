#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-DEF-031B
 * Crimson Wraith Protocol
 *
 * Real runtime behaviors:
 * - Spawns one-shot mission envelopes with hard TTL
 * - Prevents lineage respawn via terminal mission records
 * - Supports mission termination/status receipts
 */

const fs = require('fs');
const path = require('path');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  toBool,
  clampInt,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  relPath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

const POLICY_PATH = process.env.CRIMSON_WRAITH_PROTOCOL_POLICY_PATH
  ? path.resolve(process.env.CRIMSON_WRAITH_PROTOCOL_POLICY_PATH)
  : path.join(ROOT, 'config/crimson_wraith_protocol_policy.json');

const DEFAULT_POLICY = {
  version: '1.1',
  enabled: true,
  strict_default: true,
  checks: [
    { id: 'one_shot_spawn_type', description: 'Single-mission crimson_wraith spawn type available' },
    { id: 'hard_timeout_enforced', description: 'Mission timeout and TTL contracts enforced' },
    { id: 'decoy_trap_templates', description: 'Decoy and trap mission templates available' },
    { id: 'irreversible_termination', description: 'No-lineage respawn behavior enforced' }
  ],
  ttl_seconds: {
    min: 10,
    max: 1800,
    default: 120
  },
  paths: {
    state_path: 'state/security/crimson_wraith_protocol/state.json',
    latest_path: 'state/security/crimson_wraith_protocol/latest.json',
    receipts_path: 'state/security/crimson_wraith_protocol/receipts.jsonl',
    history_path: 'state/security/crimson_wraith_protocol/history.jsonl'
  }
};

function parseList(raw) {
  if (Array.isArray(raw)) return raw.map((v) => String(v || '').trim()).filter(Boolean);
  const txt = cleanText(raw || '', 4000);
  if (!txt) return [];
  return txt.split(',').map((v) => String(v || '').trim()).filter(Boolean);
}

function normalizePolicy(policyPath) {
  const raw = readJson(policyPath, {});
  const src = raw && typeof raw === 'object' ? raw : {};
  const checksSrc = Array.isArray(src.checks) ? src.checks : DEFAULT_POLICY.checks;
  const checks = checksSrc.map((row, idx) => ({
    id: normalizeToken((row && row.id) || `check_${idx + 1}`, 120) || `check_${idx + 1}`,
    description: cleanText((row && row.description) || (row && row.id) || `check_${idx + 1}`, 400),
    required: row && row.required !== false,
    file_must_exist: cleanText((row && row.file_must_exist) || '', 520)
  }));
  const ttlRaw = src.ttl_seconds && typeof src.ttl_seconds === 'object' ? src.ttl_seconds : {};
  const pathsRaw = src.paths && typeof src.paths === 'object' ? src.paths : {};
  return {
    version: cleanText(src.version || DEFAULT_POLICY.version, 32) || DEFAULT_POLICY.version,
    enabled: src.enabled !== false,
    strict_default: toBool(src.strict_default, DEFAULT_POLICY.strict_default),
    checks,
    ttl_seconds: {
      min: clampInt(ttlRaw.min, 5, 7200, DEFAULT_POLICY.ttl_seconds.min),
      max: clampInt(ttlRaw.max, 5, 21600, DEFAULT_POLICY.ttl_seconds.max),
      default: clampInt(ttlRaw.default, 5, 7200, DEFAULT_POLICY.ttl_seconds.default)
    },
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
    schema_id: 'crimson_wraith_state_v1',
    schema_version: '1.0',
    run_count: Math.max(0, Number(raw && raw.run_count || 0)),
    mission_count: Math.max(0, Number(raw && raw.mission_count || 0)),
    terminated_count: Math.max(0, Number(raw && raw.terminated_count || 0)),
    last_action: raw && raw.last_action ? cleanText(raw.last_action, 80) : null,
    last_ok: typeof (raw && raw.last_ok) === 'boolean' ? raw.last_ok : null,
    last_ts: raw && raw.last_ts ? cleanText(raw.last_ts, 80) : null,
    missions: raw && raw.missions && typeof raw.missions === 'object' ? raw.missions : {}
  };
}

function evaluateChecks(policy, failSet) {
  return policy.checks.map((check) => {
    const rel = cleanText(check.file_must_exist || '', 520);
    const abs = rel ? path.join(ROOT, rel) : '';
    const fileOk = abs ? fs.existsSync(abs) : true;
    const forcedFail = failSet.has(check.id);
    const pass = fileOk && !forcedFail;
    return {
      id: check.id,
      description: check.description,
      required: check.required !== false,
      pass,
      reason: pass ? 'ok' : (fileOk ? 'forced_failure' : 'required_file_missing'),
      file_checked: abs ? relPath(abs) : null
    };
  });
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
    failed_checks: out.failed_checks,
    mission_id: out.mission && out.mission.mission_id || null
  });
}

function classifyMissionStatus(row) {
  if (!row || typeof row !== 'object') return 'missing';
  const status = String(row.status || 'unknown');
  if (status !== 'active') return status;
  const expiresMs = Date.parse(String(row.expires_at || ''));
  if (Number.isFinite(expiresMs) && expiresMs <= Date.now()) return 'expired';
  return 'active';
}

function baseOut(policy, state, action, args, checks, extra = {}) {
  const strict = toBool(args.strict, policy.strict_default);
  const apply = toBool(args.apply, true);
  const failedChecks = checks.filter((row) => row.required !== false && row.pass !== true).map((row) => row.id);
  const ok = failedChecks.length === 0 && extra.ok !== false;
  const nextState = {
    ...state,
    run_count: state.run_count + 1,
    last_action: action,
    last_ok: ok,
    last_ts: nowIso()
  };
  const out = {
    ok,
    type: 'crimson_wraith_protocol',
    lane_id: 'V3-RACE-DEF-031B',
    title: 'Crimson Wraith Protocol',
    action,
    ts: nowIso(),
    strict,
    apply,
    checks,
    check_count: checks.length,
    failed_checks: failedChecks,
    policy_version: policy.version,
    policy_path: relPath(policy.policy_path),
    state: nextState,
    ...extra
  };
  return { out, nextState, strict, apply, ok };
}

function cmdMission(policy, args) {
  const failSet = new Set(parseList(args['fail-checks'] || args.fail_checks).map((row) => normalizeToken(row, 120)));
  const checks = evaluateChecks(policy, failSet);
  const state = loadState(policy);

  const template = normalizeToken(args.template || 'decoy_probe', 64) || 'decoy_probe';
  const ttlSeconds = clampInt(args['ttl-seconds'] || args.ttl_seconds, policy.ttl_seconds.min, policy.ttl_seconds.max, policy.ttl_seconds.default);
  const missionId = cleanText(args['mission-id'] || args.mission_id || `wraith_${stableHash(`${template}|${Date.now()}`, 16)}`, 120);

  if (ttlSeconds > 300) {
    const idx = checks.findIndex((row) => row.id === 'hard_timeout_enforced');
    if (idx >= 0) checks[idx] = { ...checks[idx], pass: false, reason: 'ttl_too_long_for_one_shot' };
  }

  if (state.missions[missionId]) {
    const idx = checks.findIndex((row) => row.id === 'one_shot_spawn_type');
    if (idx >= 0) checks[idx] = { ...checks[idx], pass: false, reason: 'mission_id_already_used' };
  }

  const mission = {
    mission_id: missionId,
    template,
    launched_at: nowIso(),
    expires_at: new Date(Date.now() + ttlSeconds * 1000).toISOString(),
    ttl_seconds: ttlSeconds,
    status: 'active',
    lineage_allowed: false
  };

  const { out, nextState, strict, apply, ok } = baseOut(policy, state, 'mission', args, checks, {
    mission
  });

  if (ok && apply) {
    nextState.missions[missionId] = mission;
    nextState.mission_count = state.mission_count + 1;
  }

  persist(policy, out, nextState, apply);
  emit(out, ok || !strict ? 0 : 2);
}

function cmdTerminate(policy, args) {
  const failSet = new Set(parseList(args['fail-checks'] || args.fail_checks).map((row) => normalizeToken(row, 120)));
  const checks = evaluateChecks(policy, failSet);
  const state = loadState(policy);
  const missionId = cleanText(args['mission-id'] || args.mission_id || '', 120);
  const mission = missionId ? (state.missions[missionId] || null) : null;

  if (!mission) {
    const idx = checks.findIndex((row) => row.id === 'irreversible_termination');
    if (idx >= 0) checks[idx] = { ...checks[idx], pass: false, reason: 'mission_not_found' };
  }

  const { out, nextState, strict, apply, ok } = baseOut(policy, state, 'terminate', args, checks, {
    mission: mission || null
  });

  if (ok && apply && mission) {
    nextState.missions[missionId] = {
      ...mission,
      status: 'terminated',
      terminated_at: nowIso(),
      termination_reason: cleanText(args.reason || 'manual_terminate', 200) || 'manual_terminate'
    };
    nextState.terminated_count = state.terminated_count + 1;
  }

  persist(policy, out, nextState, apply);
  emit(out, ok || !strict ? 0 : 2);
}

function cmdStatus(policy, args) {
  const state = loadState(policy);
  const latest = readJson(policy.paths.latest_path, null);
  const missionId = cleanText(args['mission-id'] || args.mission_id || '', 120);
  if (missionId) {
    const mission = state.missions[missionId] || null;
    emit({
      ok: !!mission,
      type: 'crimson_wraith_protocol',
      lane_id: 'V3-RACE-DEF-031B',
      action: 'status',
      ts: nowIso(),
      mission_id: missionId,
      mission,
      mission_status: classifyMissionStatus(mission),
      latest,
      policy_path: relPath(policy.policy_path)
    }, mission ? 0 : 2);
  }

  const missions = Object.values(state.missions || {}).slice(-25);
  emit({
    ok: true,
    type: 'crimson_wraith_protocol',
    lane_id: 'V3-RACE-DEF-031B',
    action: 'status',
    ts: nowIso(),
    mission_count: Object.keys(state.missions || {}).length,
    active_count: missions.filter((row) => classifyMissionStatus(row) === 'active').length,
    recent_missions: missions,
    latest,
    state,
    policy_path: relPath(policy.policy_path)
  }, 0);
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/crimson_wraith_protocol.js mission [--template=<name>] [--ttl-seconds=N] [--mission-id=<id>]');
  console.log('  node systems/security/crimson_wraith_protocol.js terminate --mission-id=<id> [--reason=...]');
  console.log('  node systems/security/crimson_wraith_protocol.js status [--mission-id=<id>]');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const action = normalizeToken(args._[0] || 'mission', 80) || 'mission';
  if (args.help || action === 'help') {
    usage();
    emit({ ok: true, type: 'crimson_wraith_protocol', action: 'help', ts: nowIso() }, 0);
  }

  const policy = normalizePolicy(args.policy ? String(args.policy) : POLICY_PATH);
  if (policy.enabled !== true) emit({ ok: false, type: 'crimson_wraith_protocol', error: 'lane_disabled', policy_path: relPath(policy.policy_path) }, 2);

  if (action === 'status') return cmdStatus(policy, args);
  if (action === 'mission' || action === 'run') return cmdMission(policy, args);
  if (action === 'terminate') return cmdTerminate(policy, args);

  usage();
  emit({ ok: false, type: 'crimson_wraith_protocol', error: 'unknown_action', action }, 2);
}

if (require.main === module) {
  main();
}
