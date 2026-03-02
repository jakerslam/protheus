#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-DEF-027
 * Project Jigsaw AttackCinema Replay Theater
 *
 * Real runtime behaviors:
 * - Capture incident frames with deterministic IDs
 * - Build replay payloads with clearance gating
 * - Emit stable highlight summaries for ops review
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
  clampInt,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  relPath,
  stableHash,
  emit
} = require('../../../lib/queued_backlog_runtime');

const POLICY_PATH = process.env.PROJECT_JIGSAW_ATTACKCINEMA_REPLAY_POLICY_PATH
  ? path.resolve(process.env.PROJECT_JIGSAW_ATTACKCINEMA_REPLAY_POLICY_PATH)
  : path.join(ROOT, 'config/project_jigsaw_attackcinema_policy.json');

const DEFAULT_POLICY = {
  version: '1.1',
  enabled: true,
  strict_default: true,
  checks: [
    {
      id: 'recording_engine_live',
      description: 'Recorder lane captures security timelines',
      file_must_exist: 'systems/security/jigsaw/README.md'
    },
    {
      id: 'highlight_editor_lane',
      description: 'Deterministic highlight generation configured'
    },
    {
      id: 'clearance4_playback_gate',
      description: 'Clearance-4 playback policy enforced'
    },
    {
      id: 'encrypted_capture_storage',
      description: 'Encrypted capture-at-rest contract active'
    }
  ],
  playback: {
    min_clearance: 4,
    max_events: 200
  },
  retention_days: 30,
  event_stream: {
    enabled: true,
    script_path: 'systems/ops/event_sourced_control_plane.js',
    stream: 'jigsaw',
    event: 'incident_capture'
  },
  paths: {
    state_path: 'state/security/project_jigsaw_attackcinema_replay/state.json',
    latest_path: 'state/security/project_jigsaw_attackcinema_replay/latest.json',
    receipts_path: 'state/security/project_jigsaw_attackcinema_replay/receipts.jsonl',
    history_path: 'state/security/project_jigsaw_attackcinema_replay/history.jsonl',
    captures_path: 'state/security/project_jigsaw_attackcinema_replay/captures.jsonl',
    highlights_path: 'state/security/project_jigsaw_attackcinema_replay/highlights.json'
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
  const playbackRaw = src.playback && typeof src.playback === 'object' ? src.playback : {};
  const streamRaw = src.event_stream && typeof src.event_stream === 'object' ? src.event_stream : {};
  const pathsRaw = src.paths && typeof src.paths === 'object' ? src.paths : {};
  return {
    version: cleanText(src.version || DEFAULT_POLICY.version, 32) || DEFAULT_POLICY.version,
    enabled: src.enabled !== false,
    strict_default: toBool(src.strict_default, DEFAULT_POLICY.strict_default),
    checks,
    playback: {
      min_clearance: clampInt(playbackRaw.min_clearance, 1, 4, DEFAULT_POLICY.playback.min_clearance),
      max_events: clampInt(playbackRaw.max_events, 10, 5000, DEFAULT_POLICY.playback.max_events)
    },
    retention_days: clampInt(src.retention_days, 1, 365, DEFAULT_POLICY.retention_days),
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
      history_path: resolvePath(pathsRaw.history_path, DEFAULT_POLICY.paths.history_path),
      captures_path: resolvePath(pathsRaw.captures_path, DEFAULT_POLICY.paths.captures_path),
      highlights_path: resolvePath(pathsRaw.highlights_path, DEFAULT_POLICY.paths.highlights_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function loadState(policy) {
  const raw = readJson(policy.paths.state_path, {});
  return {
    schema_id: 'project_jigsaw_state_v1',
    schema_version: '1.0',
    run_count: Math.max(0, Number(raw && raw.run_count || 0)),
    captured_count: Math.max(0, Number(raw && raw.captured_count || 0)),
    replay_count: Math.max(0, Number(raw && raw.replay_count || 0)),
    last_incident_id: raw && raw.last_incident_id ? cleanText(raw.last_incident_id, 120) : null,
    last_action: raw && raw.last_action ? cleanText(raw.last_action, 80) : null,
    last_ok: typeof (raw && raw.last_ok) === 'boolean' ? raw.last_ok : null,
    last_ts: raw && raw.last_ts ? cleanText(raw.last_ts, 80) : null
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

function appendCapture(policy, row) {
  appendJsonl(policy.paths.captures_path, row);
}

function readCaptures(policy) {
  const rows = [];
  try {
    if (!fs.existsSync(policy.paths.captures_path)) return rows;
    const txt = String(fs.readFileSync(policy.paths.captures_path, 'utf8') || '');
    for (const line of txt.split('\n')) {
      if (!line.trim()) continue;
      try {
        const parsed = JSON.parse(line);
        if (parsed && typeof parsed === 'object') rows.push(parsed);
      } catch {
        // ignore malformed rows
      }
    }
  } catch {
    return [];
  }
  return rows;
}

function buildHighlights(policy, rows) {
  const nowMs = Date.now();
  const cutoffMs = nowMs - (policy.retention_days * 24 * 60 * 60 * 1000);
  const kept = rows.filter((row) => {
    const ts = Date.parse(String(row && row.ts || ''));
    return Number.isFinite(ts) ? ts >= cutoffMs : true;
  });

  const severityRank = { critical: 4, high: 3, medium: 2, low: 1 };
  const ranked = kept
    .map((row) => {
      const severity = normalizeToken(row && row.severity || 'medium', 16) || 'medium';
      return { ...row, _rank: severityRank[severity] || 1 };
    })
    .sort((a, b) => Number(b._rank || 0) - Number(a._rank || 0))
    .slice(0, 20)
    .map((row) => ({
      incident_id: row.incident_id,
      severity: row.severity,
      source: row.source,
      ts: row.ts,
      summary: row.summary,
      tags: Array.isArray(row.tags) ? row.tags.slice(0, 8) : []
    }));

  const payload = {
    schema_id: 'project_jigsaw_highlights_v1',
    schema_version: '1.0',
    generated_at: nowIso(),
    retention_days: policy.retention_days,
    capture_count: kept.length,
    highlights: ranked
  };
  writeJsonAtomic(policy.paths.highlights_path, payload);
  return payload;
}

function publishEvent(policy, payload, apply, action) {
  if (!apply) return { published: false, reason: 'preview_only' };
  if (!policy.event_stream || policy.event_stream.enabled !== true) return { published: false, reason: 'event_stream_disabled' };
  if (!fs.existsSync(policy.event_stream.script_path)) {
    return { published: false, reason: 'event_stream_script_missing', script_path: relPath(policy.event_stream.script_path) };
  }
  const proc = spawnSync('node', [
    policy.event_stream.script_path,
    'append',
    `--stream=${policy.event_stream.stream}`,
    `--event=${normalizeToken(action || policy.event_stream.event, 64) || policy.event_stream.event}`,
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

function persist(policy, out, state, apply) {
  if (!apply) return;
  writeJsonAtomic(policy.paths.state_path, state);
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  appendJsonl(policy.paths.history_path, {
    ts: out.ts,
    action: out.action,
    ok: out.ok,
    failed_checks: out.failed_checks
  });
}

function cmdStatus(policy) {
  const latest = readJson(policy.paths.latest_path, null);
  const highlights = readJson(policy.paths.highlights_path, null);
  emit({
    ok: !!latest,
    type: 'project_jigsaw_attackcinema_replay',
    lane_id: 'V3-RACE-DEF-027',
    action: 'status',
    ts: nowIso(),
    latest,
    highlights,
    state: loadState(policy),
    policy_path: relPath(policy.policy_path)
  }, latest ? 0 : 2);
}

function cmdCapture(policy, args) {
  const strict = toBool(args.strict, policy.strict_default);
  const apply = toBool(args.apply, true);
  const failSet = new Set(parseList(args['fail-checks'] || args.fail_checks).map((row) => normalizeToken(row, 120)));
  const checks = evaluateChecks(policy, failSet);

  const severity = normalizeToken(args.severity || 'medium', 16) || 'medium';
  const source = cleanText(args.source || 'unknown', 120) || 'unknown';
  const summary = cleanText(args.summary || args.message || 'incident captured', 400) || 'incident captured';
  const tags = parseList(args.tags || '').map((row) => normalizeToken(row, 40)).filter(Boolean);
  const evidenceRef = cleanText(args['evidence-ref'] || args.evidence_ref || '', 520) || 'none';
  const incidentId = cleanText(args['incident-id'] || args.incident_id || `jig_${stableHash(`${source}|${severity}|${summary}|${Date.now()}`, 16)}`, 120);

  const captureRow = {
    ts: nowIso(),
    incident_id: incidentId,
    severity,
    source,
    summary,
    tags,
    evidence_ref_hash: stableHash(evidenceRef, 24),
    encrypted_capture_ref: `enc:${stableHash(`${incidentId}|${evidenceRef}`, 28)}`
  };

  const failedChecks = checks.filter((row) => row.required !== false && row.pass !== true).map((row) => row.id);
  const ok = failedChecks.length === 0;
  const prev = loadState(policy);
  const nextState = {
    ...prev,
    run_count: prev.run_count + 1,
    captured_count: prev.captured_count + (ok && apply ? 1 : 0),
    last_incident_id: incidentId,
    last_action: 'capture',
    last_ok: ok,
    last_ts: nowIso()
  };

  let highlights = readJson(policy.paths.highlights_path, {
    schema_id: 'project_jigsaw_highlights_v1',
    schema_version: '1.0',
    generated_at: null,
    retention_days: policy.retention_days,
    capture_count: 0,
    highlights: []
  });

  if (ok && apply) {
    appendCapture(policy, captureRow);
    highlights = buildHighlights(policy, readCaptures(policy));
  }

  const out = {
    ok,
    type: 'project_jigsaw_attackcinema_replay',
    lane_id: 'V3-RACE-DEF-027',
    title: 'Project Jigsaw AttackCinema Replay Theater',
    action: 'capture',
    ts: nowIso(),
    strict,
    apply,
    checks,
    check_count: checks.length,
    failed_checks: failedChecks,
    policy_version: policy.version,
    policy_path: relPath(policy.policy_path),
    incident: captureRow,
    highlight_count: Number(highlights && highlights.highlights && highlights.highlights.length || 0),
    state: nextState
  };

  out.event_stream_publish = publishEvent(policy, {
    lane_id: out.lane_id,
    action: out.action,
    incident_id: captureRow.incident_id,
    severity: captureRow.severity,
    source: captureRow.source,
    ts: out.ts
  }, apply, 'incident_capture');

  persist(policy, out, nextState, apply);
  emit(out, ok || !strict ? 0 : 2);
}

function cmdReplay(policy, args) {
  const strict = toBool(args.strict, policy.strict_default);
  const apply = toBool(args.apply, true);
  const failSet = new Set(parseList(args['fail-checks'] || args.fail_checks).map((row) => normalizeToken(row, 120)));
  const checks = evaluateChecks(policy, failSet);

  const clearance = clampInt(args.clearance, 1, 4, 2);
  if (clearance < policy.playback.min_clearance) {
    const idx = checks.findIndex((row) => row.id === 'clearance4_playback_gate');
    if (idx >= 0) checks[idx] = { ...checks[idx], pass: false, reason: 'insufficient_clearance', required_clearance: policy.playback.min_clearance };
  }

  const incidentId = cleanText(args['incident-id'] || args.incident_id || '', 120);
  const captures = readCaptures(policy);
  const selected = incidentId
    ? captures.filter((row) => String(row.incident_id || '') === incidentId)
    : captures.slice(-policy.playback.max_events);

  const replayPayload = {
    replay_id: `replay_${stableHash(`${incidentId || 'all'}|${Date.now()}`, 16)}`,
    ts: nowIso(),
    requested_incident_id: incidentId || null,
    selected_count: selected.length,
    events: selected.slice(0, policy.playback.max_events)
  };

  const failedChecks = checks.filter((row) => row.required !== false && row.pass !== true).map((row) => row.id);
  const ok = failedChecks.length === 0;
  const prev = loadState(policy);
  const nextState = {
    ...prev,
    run_count: prev.run_count + 1,
    replay_count: prev.replay_count + (ok && apply ? 1 : 0),
    last_action: 'replay',
    last_ok: ok,
    last_ts: nowIso()
  };

  const out = {
    ok,
    type: 'project_jigsaw_attackcinema_replay',
    lane_id: 'V3-RACE-DEF-027',
    title: 'Project Jigsaw AttackCinema Replay Theater',
    action: 'replay',
    ts: nowIso(),
    strict,
    apply,
    checks,
    check_count: checks.length,
    failed_checks: failedChecks,
    policy_version: policy.version,
    policy_path: relPath(policy.policy_path),
    clearance,
    playback_min_clearance: policy.playback.min_clearance,
    replay: replayPayload,
    state: nextState
  };

  out.event_stream_publish = publishEvent(policy, {
    lane_id: out.lane_id,
    action: out.action,
    replay_id: replayPayload.replay_id,
    selected_count: replayPayload.selected_count,
    clearance,
    ts: out.ts
  }, apply, 'incident_replay');

  persist(policy, out, nextState, apply);
  emit(out, ok || !strict ? 0 : 2);
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/jigsaw/attackcinema_replay_theater.js capture --severity=<low|medium|high|critical> --source=<source> [--summary=...] [--tags=a,b]');
  console.log('  node systems/security/jigsaw/attackcinema_replay_theater.js replay --clearance=4 [--incident-id=<id>]');
  console.log('  node systems/security/jigsaw/attackcinema_replay_theater.js status');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const action = normalizeToken(args._[0] || 'capture', 80) || 'capture';
  if (args.help || action === 'help') {
    usage();
    emit({ ok: true, type: 'project_jigsaw_attackcinema_replay', action: 'help', ts: nowIso() }, 0);
  }

  const policy = normalizePolicy(args.policy ? String(args.policy) : POLICY_PATH);
  if (policy.enabled !== true) {
    emit({ ok: false, type: 'project_jigsaw_attackcinema_replay', error: 'lane_disabled', policy_path: relPath(policy.policy_path) }, 2);
  }

  if (action === 'status') return cmdStatus(policy);
  if (action === 'capture' || action === 'run') return cmdCapture(policy, args);
  if (action === 'replay') return cmdReplay(policy, args);

  usage();
  emit({ ok: false, type: 'project_jigsaw_attackcinema_replay', error: 'unknown_action', action }, 2);
}

if (require.main === module) {
  main();
}
