#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-DEF-029
 * MirrorReaper Tier-4 Resource Inversion Defense Mode
 *
 * Real runtime behaviors:
 * - Calculates inversion profile from threat and capacity
 * - Enforces donor-first routing when donor capacity is available
 * - Tracks active mode state and cooldown transitions
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
  clampNumber,
  clampInt,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  relPath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

const POLICY_PATH = process.env.MIRRORREAPER_TIER4_RESOURCE_INVERSION_POLICY_PATH
  ? path.resolve(process.env.MIRRORREAPER_TIER4_RESOURCE_INVERSION_POLICY_PATH)
  : path.join(ROOT, 'config/mirrorreaper_tier4_resource_inversion_policy.json');

const DEFAULT_POLICY = {
  version: '1.1',
  enabled: true,
  strict_default: true,
  checks: [
    { id: 'tier4_activation_contract', description: 'Tier-4 activation requires corroborated signals' },
    { id: 'donor_first_compute_routing', description: 'Donor capacity preferred before local compute' },
    { id: 'mirror_workload_profiles', description: 'Mirror trap workloads scale to attacker pressure' },
    { id: 'emergency_kill_switch', description: 'Operator emergency shutdown route available' }
  ],
  thresholds: {
    activate_threat: 0.72,
    critical_threat: 0.9
  },
  cooldown_minutes: 45,
  paths: {
    state_path: 'state/security/mirrorreaper_tier4_resource_inversion/state.json',
    latest_path: 'state/security/mirrorreaper_tier4_resource_inversion/latest.json',
    receipts_path: 'state/security/mirrorreaper_tier4_resource_inversion/receipts.jsonl',
    history_path: 'state/security/mirrorreaper_tier4_resource_inversion/history.jsonl'
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
  const thresholdsRaw = src.thresholds && typeof src.thresholds === 'object' ? src.thresholds : {};
  const pathsRaw = src.paths && typeof src.paths === 'object' ? src.paths : {};
  return {
    version: cleanText(src.version || DEFAULT_POLICY.version, 32) || DEFAULT_POLICY.version,
    enabled: src.enabled !== false,
    strict_default: toBool(src.strict_default, DEFAULT_POLICY.strict_default),
    checks,
    thresholds: {
      activate_threat: clampNumber(thresholdsRaw.activate_threat, 0, 1, DEFAULT_POLICY.thresholds.activate_threat),
      critical_threat: clampNumber(thresholdsRaw.critical_threat, 0, 1, DEFAULT_POLICY.thresholds.critical_threat)
    },
    cooldown_minutes: clampInt(src.cooldown_minutes, 1, 720, DEFAULT_POLICY.cooldown_minutes),
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
    schema_id: 'mirrorreaper_state_v1',
    schema_version: '1.0',
    run_count: Math.max(0, Number(raw && raw.run_count || 0)),
    last_action: raw && raw.last_action ? cleanText(raw.last_action, 80) : null,
    last_ok: typeof (raw && raw.last_ok) === 'boolean' ? raw.last_ok : null,
    last_ts: raw && raw.last_ts ? cleanText(raw.last_ts, 80) : null,
    active_mode: cleanText(raw && raw.active_mode || 'standby', 24) || 'standby',
    cooldown_until: raw && raw.cooldown_until ? cleanText(raw.cooldown_until, 80) : null,
    last_profile_id: raw && raw.last_profile_id ? cleanText(raw.last_profile_id, 120) : null
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
    type: 'mirrorreaper_tier4_resource_inversion',
    lane_id: 'V3-RACE-DEF-029',
    title: 'MirrorReaper Tier-4 Resource Inversion Defense Mode',
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
    active_mode: out.state && out.state.active_mode || null
  });
}

function cmdActivate(policy, args) {
  const failSet = new Set(parseList(args['fail-checks'] || args.fail_checks).map((row) => normalizeToken(row, 120)));
  const checks = evaluateChecks(policy, failSet);
  const state = loadState(policy);

  const threatScore = clampNumber(args['threat-score'] || args.threat_score, 0, 1, 0.4);
  const donorCapacity = Math.max(0, Number(args['donor-capacity'] || args.donor_capacity || 0));
  const localCapacity = Math.max(0, Number(args['local-capacity'] || args.local_capacity || 0));

  if (threatScore < policy.thresholds.activate_threat) {
    const idx = checks.findIndex((row) => row.id === 'tier4_activation_contract');
    if (idx >= 0) checks[idx] = { ...checks[idx], pass: false, reason: 'threat_below_activation_threshold' };
  }
  if (donorCapacity <= 0 && localCapacity <= 0) {
    const idx = checks.findIndex((row) => row.id === 'donor_first_compute_routing');
    if (idx >= 0) checks[idx] = { ...checks[idx], pass: false, reason: 'no_capacity_available' };
  }

  const preferredRoute = donorCapacity > 0 ? 'donor_first' : 'local_fallback';
  const mode = threatScore >= policy.thresholds.critical_threat ? 'tier4_critical' : 'tier4_guarded';
  const profile = {
    profile_id: `mirror_${stableHash(`${threatScore}|${donorCapacity}|${localCapacity}|${Date.now()}`, 16)}`,
    threat_score: Number(threatScore.toFixed(6)),
    donor_capacity: Number(donorCapacity.toFixed(3)),
    local_capacity: Number(localCapacity.toFixed(3)),
    preferred_route: preferredRoute,
    workload_profile: mode === 'tier4_critical' ? 'aggressive_inversion' : 'bounded_inversion'
  };

  const { out, nextState, strict, apply, ok } = baseOut(policy, state, 'activate', args, checks, {
    profile,
    cooldown_minutes: policy.cooldown_minutes
  });

  if (ok && apply) {
    nextState.active_mode = mode;
    nextState.last_profile_id = profile.profile_id;
    nextState.cooldown_until = new Date(Date.now() + policy.cooldown_minutes * 60 * 1000).toISOString();
  }

  persist(policy, out, nextState, apply);
  emit(out, ok || !strict ? 0 : 2);
}

function cmdStandby(policy, args) {
  const failSet = new Set(parseList(args['fail-checks'] || args.fail_checks).map((row) => normalizeToken(row, 120)));
  const checks = evaluateChecks(policy, failSet);
  const state = loadState(policy);

  const { out, nextState, strict, apply, ok } = baseOut(policy, state, 'standby', args, checks, {
    transitioned_from: state.active_mode,
    transitioned_to: 'standby'
  });

  if (apply) {
    nextState.active_mode = 'standby';
    nextState.cooldown_until = null;
  }

  persist(policy, out, nextState, apply);
  emit(out, ok || !strict ? 0 : 2);
}

function cmdStatus(policy) {
  const latest = readJson(policy.paths.latest_path, null);
  emit({
    ok: !!latest,
    type: 'mirrorreaper_tier4_resource_inversion',
    lane_id: 'V3-RACE-DEF-029',
    action: 'status',
    ts: nowIso(),
    latest,
    state: loadState(policy),
    policy_path: relPath(policy.policy_path)
  }, latest ? 0 : 2);
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/mirrorreaper_tier4_resource_inversion.js activate --threat-score=0.0-1.0 [--donor-capacity=N] [--local-capacity=N]');
  console.log('  node systems/security/mirrorreaper_tier4_resource_inversion.js standby');
  console.log('  node systems/security/mirrorreaper_tier4_resource_inversion.js status');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const action = normalizeToken(args._[0] || 'activate', 80) || 'activate';
  if (args.help || action === 'help') {
    usage();
    emit({ ok: true, type: 'mirrorreaper_tier4_resource_inversion', action: 'help', ts: nowIso() }, 0);
  }

  const policy = normalizePolicy(args.policy ? String(args.policy) : POLICY_PATH);
  if (policy.enabled !== true) emit({ ok: false, type: 'mirrorreaper_tier4_resource_inversion', error: 'lane_disabled', policy_path: relPath(policy.policy_path) }, 2);

  if (action === 'status') return cmdStatus(policy);
  if (action === 'activate' || action === 'run') return cmdActivate(policy, args);
  if (action === 'standby' || action === 'deactivate') return cmdStandby(policy, args);

  usage();
  emit({ ok: false, type: 'mirrorreaper_tier4_resource_inversion', error: 'unknown_action', action }, 2);
}

if (require.main === module) {
  main();
}
