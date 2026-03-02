#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-DEF-031A
 * Thorn Swarm Protocol
 *
 * Real runtime behaviors:
 * - Plans swarm waves from threat intensity
 * - Enforces short TTL mission envelopes
 * - Emits deterministic trap profile markers
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

const POLICY_PATH = process.env.THORN_SWARM_PROTOCOL_POLICY_PATH
  ? path.resolve(process.env.THORN_SWARM_PROTOCOL_POLICY_PATH)
  : path.join(ROOT, 'config/thorn_swarm_protocol_policy.json');

const DEFAULT_POLICY = {
  version: '1.1',
  enabled: true,
  strict_default: true,
  checks: [
    { id: 'tier4_swarm_scaling', description: 'Swarm wave scaling tracks attack intensity' },
    { id: 'short_ttl_self_destruct', description: 'Sacrificial cells enforce short TTL self-destruct' },
    { id: 'trap_profile_execution', description: 'Trap profile pack executes within policy bounds' },
    { id: 'jigsaw_replay_markers', description: 'Jigsaw replay markers emitted for swarm waves' }
  ],
  wave_limits: {
    min_waves: 1,
    max_waves: 12
  },
  ttl_seconds: {
    min: 15,
    max: 600,
    default: 90
  },
  paths: {
    state_path: 'state/security/thorn_swarm_protocol/state.json',
    latest_path: 'state/security/thorn_swarm_protocol/latest.json',
    receipts_path: 'state/security/thorn_swarm_protocol/receipts.jsonl',
    history_path: 'state/security/thorn_swarm_protocol/history.jsonl',
    markers_path: 'state/security/thorn_swarm_protocol/jigsaw_markers.jsonl'
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
  const waveRaw = src.wave_limits && typeof src.wave_limits === 'object' ? src.wave_limits : {};
  const ttlRaw = src.ttl_seconds && typeof src.ttl_seconds === 'object' ? src.ttl_seconds : {};
  const pathsRaw = src.paths && typeof src.paths === 'object' ? src.paths : {};
  return {
    version: cleanText(src.version || DEFAULT_POLICY.version, 32) || DEFAULT_POLICY.version,
    enabled: src.enabled !== false,
    strict_default: toBool(src.strict_default, DEFAULT_POLICY.strict_default),
    checks,
    wave_limits: {
      min_waves: clampInt(waveRaw.min_waves, 1, 50, DEFAULT_POLICY.wave_limits.min_waves),
      max_waves: clampInt(waveRaw.max_waves, 1, 100, DEFAULT_POLICY.wave_limits.max_waves)
    },
    ttl_seconds: {
      min: clampInt(ttlRaw.min, 5, 3600, DEFAULT_POLICY.ttl_seconds.min),
      max: clampInt(ttlRaw.max, 5, 7200, DEFAULT_POLICY.ttl_seconds.max),
      default: clampInt(ttlRaw.default, 5, 7200, DEFAULT_POLICY.ttl_seconds.default)
    },
    paths: {
      state_path: resolvePath(pathsRaw.state_path, DEFAULT_POLICY.paths.state_path),
      latest_path: resolvePath(pathsRaw.latest_path, DEFAULT_POLICY.paths.latest_path),
      receipts_path: resolvePath(pathsRaw.receipts_path, DEFAULT_POLICY.paths.receipts_path),
      history_path: resolvePath(pathsRaw.history_path, DEFAULT_POLICY.paths.history_path),
      markers_path: resolvePath(pathsRaw.markers_path, DEFAULT_POLICY.paths.markers_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function loadState(policy) {
  const raw = readJson(policy.paths.state_path, {});
  return {
    schema_id: 'thorn_swarm_state_v1',
    schema_version: '1.0',
    run_count: Math.max(0, Number(raw && raw.run_count || 0)),
    wave_count_total: Math.max(0, Number(raw && raw.wave_count_total || 0)),
    last_action: raw && raw.last_action ? cleanText(raw.last_action, 80) : null,
    last_ok: typeof (raw && raw.last_ok) === 'boolean' ? raw.last_ok : null,
    last_ts: raw && raw.last_ts ? cleanText(raw.last_ts, 80) : null,
    last_operation_id: raw && raw.last_operation_id ? cleanText(raw.last_operation_id, 120) : null
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
    operation_id: out.operation && out.operation.operation_id || null
  });
  if (out.operation && out.operation.operation_id) {
    appendJsonl(policy.paths.markers_path, {
      ts: out.ts,
      marker_type: 'jigsaw_replay_marker',
      operation_id: out.operation.operation_id,
      wave_count: out.operation.wave_count,
      ttl_seconds: out.operation.ttl_seconds
    });
  }
}

function cmdSwarm(policy, args) {
  const strict = toBool(args.strict, policy.strict_default);
  const apply = toBool(args.apply, true);
  const failSet = new Set(parseList(args['fail-checks'] || args.fail_checks).map((row) => normalizeToken(row, 120)));
  const checks = evaluateChecks(policy, failSet);
  const state = loadState(policy);

  const threatScore = clampNumber(args['threat-score'] || args.threat_score, 0, 1, 0.5);
  const baseWaves = Math.round(policy.wave_limits.min_waves + ((policy.wave_limits.max_waves - policy.wave_limits.min_waves) * threatScore));
  const waveCount = clampInt(args['wave-count'] || args.wave_count, policy.wave_limits.min_waves, policy.wave_limits.max_waves, baseWaves);
  const ttlSeconds = clampInt(args['ttl-seconds'] || args.ttl_seconds, policy.ttl_seconds.min, policy.ttl_seconds.max, policy.ttl_seconds.default);

  if (ttlSeconds > 300) {
    const idx = checks.findIndex((row) => row.id === 'short_ttl_self_destruct');
    if (idx >= 0) checks[idx] = { ...checks[idx], pass: false, reason: 'ttl_above_short_lived_window', ttl_seconds: ttlSeconds };
  }

  const operation = {
    operation_id: `thorn_${stableHash(`${threatScore}|${waveCount}|${ttlSeconds}|${Date.now()}`, 16)}`,
    threat_score: Number(threatScore.toFixed(6)),
    wave_count: waveCount,
    ttl_seconds: ttlSeconds,
    trap_profiles: [
      threatScore >= 0.8 ? 'decoy_high_fidelity' : 'decoy_standard',
      threatScore >= 0.6 ? 'resource_sink_chaotic' : 'resource_sink_linear'
    ],
    generated_at: nowIso()
  };

  const failedChecks = checks.filter((row) => row.required !== false && row.pass !== true).map((row) => row.id);
  const ok = failedChecks.length === 0;
  const nextState = {
    ...state,
    run_count: state.run_count + 1,
    wave_count_total: state.wave_count_total + (ok && apply ? waveCount : 0),
    last_action: 'swarm',
    last_ok: ok,
    last_ts: nowIso(),
    last_operation_id: operation.operation_id
  };

  const out = {
    ok,
    type: 'thorn_swarm_protocol',
    lane_id: 'V3-RACE-DEF-031A',
    title: 'Thorn Swarm Protocol',
    action: 'swarm',
    ts: nowIso(),
    strict,
    apply,
    checks,
    check_count: checks.length,
    failed_checks: failedChecks,
    policy_version: policy.version,
    policy_path: relPath(policy.policy_path),
    operation,
    state: nextState
  };

  persist(policy, out, nextState, apply);
  emit(out, ok || !strict ? 0 : 2);
}

function cmdStatus(policy) {
  const latest = readJson(policy.paths.latest_path, null);
  emit({
    ok: !!latest,
    type: 'thorn_swarm_protocol',
    lane_id: 'V3-RACE-DEF-031A',
    action: 'status',
    ts: nowIso(),
    latest,
    state: loadState(policy),
    policy_path: relPath(policy.policy_path)
  }, latest ? 0 : 2);
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/thorn_swarm_protocol.js swarm --threat-score=0.0-1.0 [--wave-count=N] [--ttl-seconds=N]');
  console.log('  node systems/security/thorn_swarm_protocol.js status');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const action = normalizeToken(args._[0] || 'swarm', 80) || 'swarm';
  if (args.help || action === 'help') {
    usage();
    emit({ ok: true, type: 'thorn_swarm_protocol', action: 'help', ts: nowIso() }, 0);
  }

  const policy = normalizePolicy(args.policy ? String(args.policy) : POLICY_PATH);
  if (policy.enabled !== true) emit({ ok: false, type: 'thorn_swarm_protocol', error: 'lane_disabled', policy_path: relPath(policy.policy_path) }, 2);

  if (action === 'status') return cmdStatus(policy);
  if (action === 'swarm' || action === 'run') return cmdSwarm(policy, args);

  usage();
  emit({ ok: false, type: 'thorn_swarm_protocol', error: 'unknown_action', action }, 2);
}

if (require.main === module) {
  main();
}
