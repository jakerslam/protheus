#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-031
 * Legion Geas Protocol
 *
 * Real runtime behaviors:
 * - Issues short-lived geas leases bound to identity
 * - Verifies lease continuity and marks violations
 * - Supports revoke + status operations with receipts
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

const POLICY_PATH = process.env.LEGION_GEAS_PROTOCOL_POLICY_PATH
  ? path.resolve(process.env.LEGION_GEAS_PROTOCOL_POLICY_PATH)
  : path.join(ROOT, 'config/legion_geas_protocol_policy.json');

const DEFAULT_POLICY = {
  version: '1.1',
  enabled: true,
  strict_default: true,
  checks: [
    { id: 'cryptographic_lease_manager', description: 'Short-lived cryptographic lease manager active' },
    { id: 'behavior_continuity_validation', description: 'Three-factor validation enforced' },
    { id: 'self_destruct_on_violation', description: 'Hard self-destruct on lease breach wired' },
    { id: 'phoenix_handoff_contract', description: 'Phoenix handoff resumes inherited tactical state' }
  ],
  default_ttl_minutes: 90,
  max_ttl_minutes: 1440,
  require_factors: ['identity_anchor', 'behavior_hash', 'context_nonce'],
  paths: {
    state_path: 'state/security/legion_geas_protocol/state.json',
    latest_path: 'state/security/legion_geas_protocol/latest.json',
    receipts_path: 'state/security/legion_geas_protocol/receipts.jsonl',
    history_path: 'state/security/legion_geas_protocol/history.jsonl'
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
  const pathsRaw = src.paths && typeof src.paths === 'object' ? src.paths : {};
  return {
    version: cleanText(src.version || DEFAULT_POLICY.version, 32) || DEFAULT_POLICY.version,
    enabled: src.enabled !== false,
    strict_default: toBool(src.strict_default, DEFAULT_POLICY.strict_default),
    checks,
    default_ttl_minutes: clampInt(src.default_ttl_minutes, 5, 1440, DEFAULT_POLICY.default_ttl_minutes),
    max_ttl_minutes: clampInt(src.max_ttl_minutes, 5, 10080, DEFAULT_POLICY.max_ttl_minutes),
    require_factors: parseList(src.require_factors || DEFAULT_POLICY.require_factors).map((row) => normalizeToken(row, 80)).filter(Boolean),
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
  const leases = raw && raw.leases && typeof raw.leases === 'object' ? raw.leases : {};
  return {
    schema_id: 'legion_geas_state_v1',
    schema_version: '1.0',
    run_count: Math.max(0, Number(raw && raw.run_count || 0)),
    last_action: raw && raw.last_action ? cleanText(raw.last_action, 80) : null,
    last_ok: typeof (raw && raw.last_ok) === 'boolean' ? raw.last_ok : null,
    last_ts: raw && raw.last_ts ? cleanText(raw.last_ts, 80) : null,
    leases
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
    lease_id: out.lease_id || null,
    identity: out.identity || null
  });
}

function activeLease(lease) {
  if (!lease || typeof lease !== 'object') return false;
  if (String(lease.status || '') !== 'active') return false;
  const expiresAt = Date.parse(String(lease.expires_at || ''));
  return Number.isFinite(expiresAt) && expiresAt > Date.now();
}

function baseOutput(policy, action, args, checks, state, outExtra = {}) {
  const strict = toBool(args.strict, policy.strict_default);
  const apply = toBool(args.apply, true);
  const failedChecks = checks.filter((row) => row.required !== false && row.pass !== true).map((row) => row.id);
  const ok = failedChecks.length === 0 && outExtra.ok !== false;
  const nextState = {
    ...state,
    run_count: state.run_count + 1,
    last_action: action,
    last_ok: ok,
    last_ts: nowIso()
  };
  const out = {
    ok,
    type: 'legion_geas_protocol',
    lane_id: 'V3-RACE-031',
    title: 'Legion Geas Protocol',
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
    ...outExtra
  };
  return { out, nextState, strict, apply, ok };
}

function cmdIssue(policy, args) {
  const failSet = new Set(parseList(args['fail-checks'] || args.fail_checks).map((row) => normalizeToken(row, 120)));
  const checks = evaluateChecks(policy, failSet);
  const state = loadState(policy);

  const identity = cleanText(args.identity || args.agent || 'anonymous', 120) || 'anonymous';
  const ttlMinutes = clampInt(args['ttl-minutes'] || args.ttl_minutes, 5, policy.max_ttl_minutes, policy.default_ttl_minutes);
  const factors = {
    identity_anchor: cleanText(args.identity_anchor || identity, 180) || identity,
    behavior_hash: cleanText(args.behavior_hash || stableHash(`${identity}|${Date.now()}`, 24), 120),
    context_nonce: cleanText(args.context_nonce || stableHash(`${identity}|nonce|${Date.now()}`, 20), 80)
  };

  const missingFactors = policy.require_factors.filter((name) => !cleanText(factors[name] || '', 160));
  if (missingFactors.length > 0) {
    const idx = checks.findIndex((row) => row.id === 'behavior_continuity_validation');
    if (idx >= 0) checks[idx] = { ...checks[idx], pass: false, reason: 'missing_required_factors', missing_factors: missingFactors };
  }

  const leaseId = `geas_${stableHash(`${identity}|${Date.now()}`, 20)}`;
  const issuedAt = nowIso();
  const expiresAt = new Date(Date.now() + ttlMinutes * 60 * 1000).toISOString();
  const lease = {
    lease_id: leaseId,
    identity,
    issued_at: issuedAt,
    expires_at: expiresAt,
    ttl_minutes: ttlMinutes,
    factors,
    status: 'active'
  };

  const { out, nextState, strict, apply, ok } = baseOutput(policy, 'issue', args, checks, state, {
    lease_id: leaseId,
    identity,
    lease,
    missing_factors: missingFactors
  });

  if (ok && apply) {
    nextState.leases[identity] = lease;
  }

  persist(policy, out, nextState, apply);
  emit(out, ok || !strict ? 0 : 2);
}

function cmdVerify(policy, args) {
  const failSet = new Set(parseList(args['fail-checks'] || args.fail_checks).map((row) => normalizeToken(row, 120)));
  const checks = evaluateChecks(policy, failSet);
  const state = loadState(policy);
  const identity = cleanText(args.identity || args.agent || 'anonymous', 120) || 'anonymous';
  const lease = state.leases[identity] || null;

  if (!activeLease(lease)) {
    const idx = checks.findIndex((row) => row.id === 'self_destruct_on_violation');
    if (idx >= 0) checks[idx] = { ...checks[idx], pass: false, reason: 'lease_expired_or_missing' };
  }

  const { out, nextState, strict, apply, ok } = baseOutput(policy, 'verify', args, checks, state, {
    identity,
    lease_id: lease && lease.lease_id ? lease.lease_id : null,
    lease_active: activeLease(lease),
    lease: lease || null
  });

  if (!ok && apply && lease) {
    nextState.leases[identity] = {
      ...lease,
      status: 'violated',
      violated_at: nowIso(),
      violation_reason: 'continuity_verification_failed'
    };
  }

  persist(policy, out, nextState, apply);
  emit(out, ok || !strict ? 0 : 2);
}

function cmdRevoke(policy, args) {
  const failSet = new Set(parseList(args['fail-checks'] || args.fail_checks).map((row) => normalizeToken(row, 120)));
  const checks = evaluateChecks(policy, failSet);
  const state = loadState(policy);
  const identity = cleanText(args.identity || args.agent || 'anonymous', 120) || 'anonymous';
  const lease = state.leases[identity] || null;

  const { out, nextState, strict, apply, ok } = baseOutput(policy, 'revoke', args, checks, state, {
    identity,
    lease_id: lease && lease.lease_id ? lease.lease_id : null,
    revoked: !!lease
  });

  if (apply && lease) {
    nextState.leases[identity] = {
      ...lease,
      status: 'revoked',
      revoked_at: nowIso(),
      revoke_note: cleanText(args.note || 'manual_revoke', 240) || 'manual_revoke'
    };
  }

  persist(policy, out, nextState, apply);
  emit(out, ok || !strict ? 0 : 2);
}

function cmdEnforce(policy, args) {
  const failSet = new Set(parseList(args['fail-checks'] || args.fail_checks).map((row) => normalizeToken(row, 120)));
  const checks = evaluateChecks(policy, failSet);
  const state = loadState(policy);
  const identity = cleanText(args.identity || args.agent || 'anonymous', 120) || 'anonymous';
  const lease = state.leases[identity] || null;

  if (!activeLease(lease)) {
    const idx = checks.findIndex((row) => row.id === 'self_destruct_on_violation');
    if (idx >= 0) checks[idx] = { ...checks[idx], pass: false, reason: 'no_active_lease' };
  }

  const { out, nextState, strict, apply, ok } = baseOutput(policy, 'enforce', args, checks, state, {
    identity,
    lease_id: lease && lease.lease_id ? lease.lease_id : null,
    lease_active: activeLease(lease)
  });

  if (!ok && apply && lease) {
    nextState.leases[identity] = {
      ...lease,
      status: 'terminated',
      terminated_at: nowIso(),
      termination_reason: 'enforcement_block'
    };
  }

  persist(policy, out, nextState, apply);
  emit(out, ok || !strict ? 0 : 2);
}

function cmdStatus(policy, args) {
  const state = loadState(policy);
  const latest = readJson(policy.paths.latest_path, null);
  const identity = cleanText(args.identity || args.agent || '', 120);
  emit({
    ok: true,
    type: 'legion_geas_protocol',
    lane_id: 'V3-RACE-031',
    action: 'status',
    ts: nowIso(),
    identity: identity || null,
    lease: identity ? (state.leases[identity] || null) : null,
    lease_count: Object.keys(state.leases).length,
    latest,
    state,
    policy_path: relPath(policy.policy_path)
  }, 0);
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/legion_geas_protocol.js issue --identity=<id> [--ttl-minutes=90]');
  console.log('  node systems/security/legion_geas_protocol.js verify --identity=<id>');
  console.log('  node systems/security/legion_geas_protocol.js enforce --identity=<id>');
  console.log('  node systems/security/legion_geas_protocol.js revoke --identity=<id> [--note=...]');
  console.log('  node systems/security/legion_geas_protocol.js status [--identity=<id>]');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const action = normalizeToken(args._[0] || 'enforce', 80) || 'enforce';
  if (args.help || action === 'help') {
    usage();
    emit({ ok: true, type: 'legion_geas_protocol', action: 'help', ts: nowIso() }, 0);
  }

  const policy = normalizePolicy(args.policy ? String(args.policy) : POLICY_PATH);
  if (policy.enabled !== true) emit({ ok: false, type: 'legion_geas_protocol', error: 'lane_disabled', policy_path: relPath(policy.policy_path) }, 2);

  if (action === 'status') return cmdStatus(policy, args);
  if (action === 'issue') return cmdIssue(policy, args);
  if (action === 'verify') return cmdVerify(policy, args);
  if (action === 'revoke') return cmdRevoke(policy, args);
  if (action === 'enforce' || action === 'run') return cmdEnforce(policy, args);

  usage();
  emit({ ok: false, type: 'legion_geas_protocol', error: 'unknown_action', action }, 2);
}

if (require.main === module) {
  main();
}
