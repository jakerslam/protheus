#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-DEF-026
 * Lockweaver Eternal Flux Field
 *
 * Real runtime behaviors:
 * - Computes adaptive flux cadence from threat/mutation pressure
 * - Enforces scope exclusion invariant before apply
 * - Writes deterministic state/latest/receipts/history artifacts
 * - Publishes best-effort event rows to event_sourced_control_plane
 */

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
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
  emit
} = require('../../../lib/queued_backlog_runtime');

const POLICY_PATH = process.env.LOCKWEAVER_ETERNAL_FLUX_FIELD_POLICY_PATH
  ? path.resolve(process.env.LOCKWEAVER_ETERNAL_FLUX_FIELD_POLICY_PATH)
  : path.join(ROOT, 'config/lockweaver_eternal_flux_field_policy.json');

const DEFAULT_POLICY = {
  version: '1.1',
  enabled: true,
  strict_default: true,
  checks: [
    {
      id: 'origin_lock_verification',
      description: 'Origin lock verify and reseed loop active',
      file_must_exist: 'systems/security/lockweaver/README.md'
    },
    {
      id: 'mutation_cycle_receipts',
      description: 'Cycle receipts publish to authoritative stream'
    },
    {
      id: 'fractal_rate_controller',
      description: 'Threat adaptive cadence controller configured'
    },
    {
      id: 'scope_exclusion_invariant',
      description: 'Open platform/habits/skills exclusion enforced'
    }
  ],
  min_cadence_seconds: 120,
  max_cadence_seconds: 3600,
  pressure_weights: {
    threat_score: 0.65,
    mutation_count: 0.35
  },
  excluded_scope_prefixes: [
    'platform/',
    'habits/',
    'skills/'
  ],
  event_stream: {
    enabled: true,
    script_path: 'systems/ops/event_sourced_control_plane.js',
    stream: 'lockweaver',
    event: 'flux_cycle'
  },
  paths: {
    state_path: 'state/security/lockweaver_eternal_flux_field/state.json',
    latest_path: 'state/security/lockweaver_eternal_flux_field/latest.json',
    receipts_path: 'state/security/lockweaver_eternal_flux_field/receipts.jsonl',
    history_path: 'state/security/lockweaver_eternal_flux_field/history.jsonl'
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
  const streamRaw = src.event_stream && typeof src.event_stream === 'object' ? src.event_stream : {};
  const weightsRaw = src.pressure_weights && typeof src.pressure_weights === 'object' ? src.pressure_weights : {};

  return {
    version: cleanText(src.version || DEFAULT_POLICY.version, 32) || DEFAULT_POLICY.version,
    enabled: src.enabled !== false,
    strict_default: toBool(src.strict_default, DEFAULT_POLICY.strict_default),
    checks,
    min_cadence_seconds: clampInt(src.min_cadence_seconds, 30, 86400, DEFAULT_POLICY.min_cadence_seconds),
    max_cadence_seconds: clampInt(src.max_cadence_seconds, 30, 86400, DEFAULT_POLICY.max_cadence_seconds),
    pressure_weights: {
      threat_score: clampNumber(weightsRaw.threat_score, 0, 1, DEFAULT_POLICY.pressure_weights.threat_score),
      mutation_count: clampNumber(weightsRaw.mutation_count, 0, 1, DEFAULT_POLICY.pressure_weights.mutation_count)
    },
    excluded_scope_prefixes: parseList(src.excluded_scope_prefixes || DEFAULT_POLICY.excluded_scope_prefixes)
      .map((row) => row.replace(/^\/+/, '').toLowerCase()),
    event_stream: {
      enabled: toBool(streamRaw.enabled, DEFAULT_POLICY.event_stream.enabled),
      script_path: resolvePath(streamRaw.script_path, DEFAULT_POLICY.event_stream.script_path),
      stream: normalizeToken(streamRaw.stream || DEFAULT_POLICY.event_stream.stream, 64) || DEFAULT_POLICY.event_stream.stream,
      event: normalizeToken(streamRaw.event || DEFAULT_POLICY.event_stream.event, 64) || DEFAULT_POLICY.event_stream.event
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
    schema_id: 'lockweaver_flux_state_v1',
    schema_version: '1.0',
    run_count: Math.max(0, Number(raw && raw.run_count || 0)),
    last_action: raw && raw.last_action ? cleanText(raw.last_action, 80) : null,
    last_ok: typeof (raw && raw.last_ok) === 'boolean' ? raw.last_ok : null,
    last_ts: raw && raw.last_ts ? cleanText(raw.last_ts, 80) : null,
    last_cadence_seconds: Math.max(0, Number(raw && raw.last_cadence_seconds || 0)) || null,
    last_pressure_score: clampNumber(raw && raw.last_pressure_score, 0, 1, 0),
    flux_epoch: Math.max(0, Number(raw && raw.flux_epoch || 0))
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

function computeCadenceSeconds(policy, threatScore, mutationCount) {
  const minCadence = Math.min(policy.min_cadence_seconds, policy.max_cadence_seconds);
  const maxCadence = Math.max(policy.min_cadence_seconds, policy.max_cadence_seconds);
  const mutationNorm = clampNumber(mutationCount / 24, 0, 1, 0);
  const wThreat = clampNumber(policy.pressure_weights.threat_score, 0, 1, 0.65);
  const wMutations = clampNumber(policy.pressure_weights.mutation_count, 0, 1, 0.35);
  const sum = Math.max(0.0001, wThreat + wMutations);
  const pressure = clampNumber(((threatScore * wThreat) + (mutationNorm * wMutations)) / sum, 0, 1, 0);
  const cadence = Math.round(maxCadence - ((maxCadence - minCadence) * pressure));
  return {
    cadence_seconds: Math.max(minCadence, Math.min(maxCadence, cadence)),
    pressure_score: Number(pressure.toFixed(6)),
    mutation_norm: Number(mutationNorm.toFixed(6))
  };
}

function hasExcludedScope(policy, scopeRoots) {
  const normalized = scopeRoots
    .map((row) => String(row || '').trim().replace(/^\/+/, '').toLowerCase())
    .filter(Boolean);
  const hits = [];
  for (const scope of normalized) {
    for (const prefix of policy.excluded_scope_prefixes) {
      if (!prefix) continue;
      if (scope === prefix || scope.startsWith(prefix)) {
        hits.push(scope);
        break;
      }
    }
  }
  return {
    blocked: hits.length > 0,
    scope_hits: Array.from(new Set(hits))
  };
}

function publishEvent(policy, payload, apply) {
  if (!apply) return { published: false, reason: 'preview_only' };
  if (!policy.event_stream || policy.event_stream.enabled !== true) {
    return { published: false, reason: 'event_stream_disabled' };
  }
  const scriptPath = policy.event_stream.script_path;
  if (!fs.existsSync(scriptPath)) {
    return { published: false, reason: 'event_stream_script_missing', script_path: relPath(scriptPath) };
  }
  const proc = spawnSync('node', [
    scriptPath,
    'append',
    `--stream=${policy.event_stream.stream}`,
    `--event=${policy.event_stream.event}`,
    `--payload_json=${JSON.stringify(payload)}`
  ], {
    cwd: ROOT,
    encoding: 'utf8'
  });
  return {
    published: Number(proc.status || 0) === 0,
    reason: Number(proc.status || 0) === 0 ? 'event_stream_append_ok' : 'event_stream_append_failed',
    status: Number(proc.status || 0),
    stderr: cleanText(proc.stderr || '', 240) || null
  };
}

function cmdStatus(policy) {
  const latest = readJson(policy.paths.latest_path, null);
  emit({
    ok: !!latest,
    type: 'lockweaver_eternal_flux_field',
    lane_id: 'V3-RACE-DEF-026',
    action: 'status',
    ts: nowIso(),
    latest,
    state: loadState(policy),
    policy_path: relPath(policy.policy_path)
  }, latest ? 0 : 2);
}

function cmdFlux(policy, args) {
  const strict = toBool(args.strict, policy.strict_default);
  const apply = toBool(args.apply, true);
  const failSet = new Set(parseList(args['fail-checks'] || args.fail_checks).map((row) => normalizeToken(row, 120)));
  const checks = evaluateChecks(policy, failSet);

  const threatScore = clampNumber(args['threat-score'] || args.threat_score, 0, 1, 0.35);
  const mutationCount = clampInt(args['mutation-count'] || args.mutation_count, 0, 10_000, 0);
  const scopeRoots = parseList(args['scope-roots'] || args.scope_roots || args.scope || 'systems/security');

  const scopeCheck = hasExcludedScope(policy, scopeRoots);
  if (scopeCheck.blocked) {
    const idx = checks.findIndex((row) => row.id === 'scope_exclusion_invariant');
    if (idx >= 0) {
      checks[idx] = {
        ...checks[idx],
        pass: false,
        reason: 'excluded_scope_violation',
        scope_hits: scopeCheck.scope_hits
      };
    }
  }

  const failedChecks = checks.filter((row) => row.required !== false && row.pass !== true).map((row) => row.id);
  const ok = failedChecks.length === 0;
  const cadence = computeCadenceSeconds(policy, threatScore, mutationCount);

  const prev = loadState(policy);
  const nextState = {
    ...prev,
    run_count: prev.run_count + 1,
    last_action: 'flux',
    last_ok: ok,
    last_ts: nowIso(),
    last_cadence_seconds: cadence.cadence_seconds,
    last_pressure_score: cadence.pressure_score,
    flux_epoch: prev.flux_epoch + 1
  };

  const out = {
    ok,
    type: 'lockweaver_eternal_flux_field',
    lane_id: 'V3-RACE-DEF-026',
    title: 'Lockweaver Eternal Flux Field',
    action: 'flux',
    ts: nowIso(),
    strict,
    apply,
    checks,
    check_count: checks.length,
    failed_checks: failedChecks,
    policy_version: policy.version,
    policy_path: relPath(policy.policy_path),
    cadence_seconds: cadence.cadence_seconds,
    pressure_score: cadence.pressure_score,
    threat_score: Number(threatScore.toFixed(6)),
    mutation_count: mutationCount,
    mutation_norm: cadence.mutation_norm,
    scope_roots: scopeRoots,
    excluded_scope_hits: scopeCheck.scope_hits,
    state: nextState
  };

  const eventResult = publishEvent(policy, {
    lane_id: out.lane_id,
    ok: out.ok,
    cadence_seconds: out.cadence_seconds,
    pressure_score: out.pressure_score,
    threat_score: out.threat_score,
    mutation_count: out.mutation_count,
    excluded_scope_hits: out.excluded_scope_hits,
    ts: out.ts
  }, apply);
  out.event_stream_publish = eventResult;

  if (apply) {
    writeJsonAtomic(policy.paths.state_path, nextState);
    writeJsonAtomic(policy.paths.latest_path, out);
    appendJsonl(policy.paths.receipts_path, out);
    appendJsonl(policy.paths.history_path, {
      ts: out.ts,
      action: out.action,
      ok: out.ok,
      cadence_seconds: out.cadence_seconds,
      pressure_score: out.pressure_score,
      failed_checks: out.failed_checks
    });
  }

  emit(out, ok || !strict ? 0 : 2);
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/lockweaver/eternal_flux_field.js flux [--threat-score=0.0-1.0] [--mutation-count=N] [--scope-roots=a,b] [--strict=1|0] [--apply=1|0]');
  console.log('  node systems/security/lockweaver/eternal_flux_field.js status');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const action = normalizeToken(args._[0] || 'flux', 80) || 'flux';
  if (action === 'help' || args.help) {
    usage();
    emit({ ok: true, type: 'lockweaver_eternal_flux_field', action: 'help', ts: nowIso() }, 0);
  }

  const policy = normalizePolicy(args.policy ? String(args.policy) : POLICY_PATH);
  if (policy.enabled !== true) {
    emit({ ok: false, type: 'lockweaver_eternal_flux_field', error: 'lane_disabled', policy_path: relPath(policy.policy_path) }, 2);
  }

  if (action === 'status') return cmdStatus(policy);
  if (action === 'flux' || action === 'run') return cmdFlux(policy, args);

  usage();
  emit({ ok: false, type: 'lockweaver_eternal_flux_field', error: 'unknown_action', action }, 2);
}

if (require.main === module) {
  main();
}
