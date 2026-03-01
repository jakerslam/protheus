#!/usr/bin/env node
'use strict';
export {};

/** V3-RACE-003 + V3-RACE-017 */
const path = require('path');
const { spawnSync } = require('child_process');
const {
  ROOT, nowIso, parseArgs, normalizeToken, toBool, readJson,
  readJsonl, writeJsonAtomic, appendJsonl, resolvePath, stableHash, emit
} = require('../../lib/queued_backlog_runtime');

const POLICY_PATH = process.env.EVENT_SOURCED_CONTROL_PLANE_POLICY_PATH
  ? path.resolve(process.env.EVENT_SOURCED_CONTROL_PLANE_POLICY_PATH)
  : path.join(ROOT, 'config', 'event_sourced_control_plane_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/ops/event_sourced_control_plane.js append --stream=<id> --event=<id> [--payload_json={}]');
  console.log('  node systems/ops/event_sourced_control_plane.js set-authority --source=local_authority|stream_authority [--reason=<text>] [--apply=1|0]');
  console.log('  node systems/ops/event_sourced_control_plane.js rebuild [--source=local_authority|stream_authority]');
  console.log('  node systems/ops/event_sourced_control_plane.js reconcile [--partition=1|0] [--strict=1|0] [--apply=1|0]');
  console.log('  node systems/ops/event_sourced_control_plane.js replay');
  console.log('  node systems/ops/event_sourced_control_plane.js status');
}

function policy() {
  const base = {
    enabled: true,
    shadow_only: true,
    authority: {
      source: 'local_authority',
      strict_reconcile: true,
      rollback_on_partition: true
    },
    jetstream: {
      enabled: false,
      shadow_only: true,
      allow_shadow_publish: false,
      subject_prefix: 'protheus.events',
      publish_command: ['nats', 'pub'],
      timeout_ms: 5000
    },
    paths: {
      events_path: 'state/ops/event_sourced_control_plane/events.jsonl',
      stream_events_path: 'state/ops/event_sourced_control_plane/stream_events.jsonl',
      views_path: 'state/ops/event_sourced_control_plane/materialized_views.json',
      latest_path: 'state/ops/event_sourced_control_plane/latest.json',
      receipts_path: 'state/ops/event_sourced_control_plane/receipts.jsonl',
      jetstream_latest_path: 'state/ops/event_sourced_control_plane/jetstream_latest.json',
      authority_state_path: 'state/ops/event_sourced_control_plane/authority_state.json',
      reconcile_latest_path: 'state/ops/event_sourced_control_plane/reconcile_latest.json',
      rollback_latest_path: 'state/ops/event_sourced_control_plane/rollback_latest.json'
    }
  };
  const raw = readJson(POLICY_PATH, {});
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  const jetstream = raw.jetstream && typeof raw.jetstream === 'object' ? raw.jetstream : {};
  const authority = raw.authority && typeof raw.authority === 'object' ? raw.authority : {};
  const publishCommand = Array.isArray(jetstream.publish_command) && jetstream.publish_command.length > 0
    ? jetstream.publish_command.map((row: unknown) => String(row || '').trim()).filter(Boolean)
    : base.jetstream.publish_command;
  return {
    enabled: toBool(raw.enabled, base.enabled),
    shadow_only: toBool(raw.shadow_only, base.shadow_only),
    authority: {
      source: normalizeAuthoritySource(authority.source || base.authority.source),
      strict_reconcile: toBool(authority.strict_reconcile, base.authority.strict_reconcile),
      rollback_on_partition: toBool(authority.rollback_on_partition, base.authority.rollback_on_partition)
    },
    jetstream: {
      enabled: toBool(jetstream.enabled, base.jetstream.enabled),
      shadow_only: toBool(jetstream.shadow_only, base.jetstream.shadow_only),
      allow_shadow_publish: toBool(jetstream.allow_shadow_publish, base.jetstream.allow_shadow_publish),
      subject_prefix: normalizeToken(jetstream.subject_prefix || base.jetstream.subject_prefix, 120) || base.jetstream.subject_prefix,
      publish_command: publishCommand,
      timeout_ms: Number.isFinite(Number(jetstream.timeout_ms))
        ? Math.max(1000, Math.floor(Number(jetstream.timeout_ms)))
        : base.jetstream.timeout_ms
    },
    paths: {
      events_path: resolvePath(paths.events_path, base.paths.events_path),
      stream_events_path: resolvePath(paths.stream_events_path, base.paths.stream_events_path),
      views_path: resolvePath(paths.views_path, base.paths.views_path),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path),
      jetstream_latest_path: resolvePath(paths.jetstream_latest_path, base.paths.jetstream_latest_path),
      authority_state_path: resolvePath(paths.authority_state_path, base.paths.authority_state_path),
      reconcile_latest_path: resolvePath(paths.reconcile_latest_path, base.paths.reconcile_latest_path),
      rollback_latest_path: resolvePath(paths.rollback_latest_path, base.paths.rollback_latest_path)
    }
  };
}

function normalizeAuthoritySource(raw: any) {
  const v = normalizeToken(raw || 'local_authority', 64) || 'local_authority';
  return v === 'stream_authority' ? 'stream_authority' : 'local_authority';
}

function loadAuthorityState(p: any) {
  const src = readJson(p.paths.authority_state_path, null);
  if (!src || typeof src !== 'object') {
    return {
      schema_id: 'event_sourced_control_plane_authority_state',
      schema_version: '1.0',
      source: normalizeAuthoritySource(p.authority && p.authority.source || 'local_authority'),
      updated_at: nowIso(),
      last_reason: 'policy_default',
      rollback_count: 0,
      reconcile_failures: 0
    };
  }
  return {
    schema_id: 'event_sourced_control_plane_authority_state',
    schema_version: '1.0',
    source: normalizeAuthoritySource(src.source || p.authority.source || 'local_authority'),
    updated_at: src.updated_at || nowIso(),
    last_reason: String(src.last_reason || 'state_loaded').slice(0, 200),
    rollback_count: Math.max(0, Number(src.rollback_count || 0)),
    reconcile_failures: Math.max(0, Number(src.reconcile_failures || 0))
  };
}

function saveAuthorityState(p: any, state: any) {
  writeJsonAtomic(p.paths.authority_state_path, {
    schema_id: 'event_sourced_control_plane_authority_state',
    schema_version: '1.0',
    source: normalizeAuthoritySource(state.source || 'local_authority'),
    updated_at: nowIso(),
    last_reason: String(state.last_reason || 'updated').slice(0, 200),
    rollback_count: Math.max(0, Number(state.rollback_count || 0)),
    reconcile_failures: Math.max(0, Number(state.reconcile_failures || 0))
  });
}

function mirrorToJetStream(event: any, p: any) {
  const cfg = p.jetstream || {};
  const stream = normalizeToken(event && event.stream || 'control', 80) || 'control';
  const evt = normalizeToken(event && event.event || 'mutation', 80) || 'mutation';
  const subject = `${cfg.subject_prefix || 'protheus.events'}.${stream}.${evt}`;
  const payload = JSON.stringify({
    schema_id: 'event_sourced_control_plane_mirror',
    schema_version: '1.0',
    mirrored_at: nowIso(),
    event
  });
  const payload_hash = stableHash(payload, 32);

  if (cfg.enabled !== true) {
    return { mirrored: false, reason: 'jetstream_disabled', subject, payload_hash };
  }
  if (cfg.shadow_only === true && cfg.allow_shadow_publish !== true) {
    return { mirrored: false, reason: 'jetstream_shadow_only_simulated', subject, payload_hash };
  }
  const cmd = Array.isArray(cfg.publish_command) ? cfg.publish_command.filter(Boolean) : [];
  if (cmd.length < 1) {
    return { mirrored: false, reason: 'jetstream_publish_command_missing', subject, payload_hash };
  }

  const proc = spawnSync(cmd[0], cmd.slice(1).concat([subject, payload]), {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: Number(cfg.timeout_ms || 5000)
  });
  const status = Number(proc.status || 0);
  const ok = status === 0;
  const out = {
    mirrored: ok,
    reason: ok ? 'jetstream_publish_ok' : 'jetstream_publish_failed',
    subject,
    payload_hash,
    command: cmd,
    status,
    stderr: String(proc.stderr || '').trim().slice(0, 600)
  };
  writeJsonAtomic(p.paths.jetstream_latest_path, {
    ts: nowIso(),
    type: 'event_sourced_control_plane_jetstream_mirror',
    ...out
  });
  return out;
}

function parsePayload(raw: any) {
  if (!raw) return {};
  try { return JSON.parse(String(raw)); } catch { return { raw: String(raw).slice(0, 2000) }; }
}

function toStreamEventRow(event: any, jetstreamMirror: any) {
  return {
    ts: nowIso(),
    type: 'event_stream_authority_row',
    stream_event_id: `sevt_${Date.now()}_${stableHash(event.event_id || '', 10)}`,
    local_event_id: event.event_id,
    stream: event.stream,
    event: {
      ...event,
      payload_hash: stableHash(JSON.stringify(event.payload || {}), 24)
    },
    jetstream: jetstreamMirror
  };
}

function normalizeEvent(row: any) {
  const src = row && row.event && typeof row.event === 'object' ? row.event : row;
  if (!src || typeof src !== 'object') return null;
  const eventId = normalizeToken(src.event_id || src.local_event_id || '', 120);
  if (!eventId) return null;
  return {
    ts: src.ts || null,
    event_id: eventId,
    stream: normalizeToken(src.stream || 'control', 80) || 'control',
    event: normalizeToken(src.event || 'mutation', 80) || 'mutation',
    payload: src.payload && typeof src.payload === 'object' ? src.payload : {},
    payload_hash: src.payload_hash || stableHash(JSON.stringify(src.payload || {}), 24)
  };
}

function readAuthorityEvents(p: any, source: string) {
  const fromSource = normalizeAuthoritySource(source);
  const rows = fromSource === 'stream_authority'
    ? readJsonl(p.paths.stream_events_path)
    : readJsonl(p.paths.events_path);
  const out = [];
  for (const row of rows) {
    const evt = normalizeEvent(row);
    if (evt) out.push(evt);
  }
  return out;
}

function buildMaterializedView(events: any[], source: string, reason = 'materialize') {
  const byStream: Record<string, any> = {};
  for (const row of events) {
    const stream = normalizeToken(row.stream || 'control', 80) || 'control';
    if (!byStream[stream]) {
      byStream[stream] = {
        stream,
        events: 0,
        last_event: null,
        last_event_id: null
      };
    }
    byStream[stream].events += 1;
    byStream[stream].last_event = row.event;
    byStream[stream].last_event_id = row.event_id;
  }
  const signature = stableHash(JSON.stringify({
    source: normalizeAuthoritySource(source),
    ids: events.map((row) => row.event_id)
  }), 24);
  return {
    schema_id: 'event_sourced_control_plane_materialized_view',
    schema_version: '1.0',
    generated_at: nowIso(),
    reason,
    authority_source: normalizeAuthoritySource(source),
    event_count: events.length,
    stream_count: Object.keys(byStream).length,
    signature,
    streams: Object.values(byStream)
  };
}

function appendEvent(args: any, p: any) {
  const event = {
    ts: nowIso(),
    event_id: `evt_${Date.now()}_${stableHash(JSON.stringify(args), 8)}`,
    stream: normalizeToken(args.stream || 'control', 80) || 'control',
    event: normalizeToken(args.event || 'mutation', 80) || 'mutation',
    payload: parsePayload(args.payload_json)
  };
  event.payload_hash = stableHash(JSON.stringify(event.payload || {}), 24);
  appendJsonl(p.paths.events_path, event);

  const jetstreamMirror = mirrorToJetStream(event, p);
  appendJsonl(p.paths.stream_events_path, toStreamEventRow(event, jetstreamMirror));

  const authorityState = loadAuthorityState(p);
  const authorityEvents = readAuthorityEvents(p, authorityState.source);
  const view = buildMaterializedView(authorityEvents, authorityState.source, 'append');
  writeJsonAtomic(p.paths.views_path, view);

  const receipt = {
    ts: nowIso(),
    type: 'event_sourced_control_plane_append',
    ok: true,
    shadow_only: p.shadow_only,
    event_id: event.event_id,
    stream: event.stream,
    authority_source: authorityState.source,
    materialized_streams: view.stream_count,
    materialized_event_count: view.event_count,
    materialized_signature: view.signature,
    jetstream: jetstreamMirror
  };
  writeJsonAtomic(p.paths.latest_path, receipt);
  appendJsonl(p.paths.receipts_path, receipt);
  return receipt;
}

function cmdSetAuthority(args: any, p: any) {
  const source = normalizeAuthoritySource(args.source || '');
  const apply = toBool(args.apply, true);
  const reason = String(args.reason || 'manual_set_authority').slice(0, 220);
  const state = loadAuthorityState(p);
  const previous = state.source;
  if (apply) {
    state.source = source;
    state.last_reason = reason;
    saveAuthorityState(p, state);
  }
  const out = {
    ts: nowIso(),
    type: 'event_sourced_control_plane_set_authority',
    ok: true,
    apply,
    previous_source: previous,
    next_source: source,
    strict_reconcile: p.authority.strict_reconcile === true,
    rollback_on_partition: p.authority.rollback_on_partition === true,
    reason
  };
  writeJsonAtomic(p.paths.latest_path, out);
  appendJsonl(p.paths.receipts_path, out);
  return out;
}

function cmdRebuild(args: any, p: any) {
  const state = loadAuthorityState(p);
  const source = normalizeAuthoritySource(args.source || state.source);
  const events = readAuthorityEvents(p, source);
  const view = buildMaterializedView(events, source, 'rebuild');
  writeJsonAtomic(p.paths.views_path, view);
  const out = {
    ts: nowIso(),
    type: 'event_sourced_control_plane_rebuild',
    ok: true,
    source,
    event_count: view.event_count,
    stream_count: view.stream_count,
    signature: view.signature
  };
  writeJsonAtomic(p.paths.latest_path, out);
  appendJsonl(p.paths.receipts_path, out);
  return out;
}

function cmdReconcile(args: any, p: any) {
  const strict = toBool(args.strict, false);
  const apply = toBool(args.apply, true);
  const partition = toBool(args.partition, false);

  const local = readAuthorityEvents(p, 'local_authority');
  const stream = readAuthorityEvents(p, 'stream_authority');
  const localMap: Record<string, any> = {};
  const streamMap: Record<string, any> = {};

  for (const row of local) localMap[row.event_id] = row;
  for (const row of stream) streamMap[row.event_id] = row;

  const localIds = Object.keys(localMap).sort();
  const streamIds = Object.keys(streamMap).sort();
  const missingInStream = localIds.filter((id) => !streamMap[id]);
  const missingInLocal = streamIds.filter((id) => !localMap[id]);
  const payloadMismatch = [];
  for (const id of localIds) {
    if (!streamMap[id]) continue;
    if (String(localMap[id].payload_hash || '') !== String(streamMap[id].payload_hash || '')) {
      payloadMismatch.push(id);
    }
  }

  const mismatchCount = missingInStream.length + missingInLocal.length + payloadMismatch.length;
  const pass = mismatchCount === 0;
  const reconcile = {
    ts: nowIso(),
    type: 'event_sourced_control_plane_reconcile',
    ok: pass,
    strict,
    apply,
    partition,
    local_event_count: localIds.length,
    stream_event_count: streamIds.length,
    mismatch_count: mismatchCount,
    missing_in_stream: missingInStream.slice(0, 20),
    missing_in_local: missingInLocal.slice(0, 20),
    payload_mismatch: payloadMismatch.slice(0, 20),
    signatures: {
      local: stableHash(localIds.join('|'), 20),
      stream: stableHash(streamIds.join('|'), 20)
    },
    rollback: null as any
  };

  const state = loadAuthorityState(p);
  if (!pass) {
    state.reconcile_failures = Math.max(0, Number(state.reconcile_failures || 0)) + 1;
  }

  if (!pass && partition && apply && p.authority.rollback_on_partition === true) {
    const from = state.source;
    state.source = 'local_authority';
    state.last_reason = 'partition_reconcile_mismatch_auto_rollback';
    state.rollback_count = Math.max(0, Number(state.rollback_count || 0)) + 1;
    reconcile.rollback = {
      ok: true,
      from_source: from,
      to_source: 'local_authority',
      reason: 'partition_reconcile_mismatch_auto_rollback'
    };
    writeJsonAtomic(p.paths.rollback_latest_path, {
      ts: nowIso(),
      type: 'event_sourced_control_plane_authority_rollback',
      ...reconcile.rollback
    });
  }

  if (apply) saveAuthorityState(p, state);
  writeJsonAtomic(p.paths.reconcile_latest_path, reconcile);
  writeJsonAtomic(p.paths.latest_path, reconcile);
  appendJsonl(p.paths.receipts_path, reconcile);

  if (strict && !pass) emit(reconcile, 1);
  return reconcile;
}

function replay(p: any) {
  const state = loadAuthorityState(p);
  const events = readAuthorityEvents(p, state.source);
  const streams: Record<string, number> = {};
  for (const row of events) {
    const stream = normalizeToken(row.stream || 'control', 80) || 'control';
    streams[stream] = (streams[stream] || 0) + 1;
  }
  const view = buildMaterializedView(events, state.source, 'replay');
  writeJsonAtomic(p.paths.views_path, view);
  const receipt = {
    ts: nowIso(),
    type: 'event_sourced_control_plane_replay',
    ok: true,
    shadow_only: p.shadow_only,
    authority_source: state.source,
    replay_event_count: events.length,
    stream_count: Object.keys(streams).length,
    signature: view.signature
  };
  writeJsonAtomic(p.paths.latest_path, receipt);
  appendJsonl(p.paths.receipts_path, receipt);
  return receipt;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 80) || 'status';
  if (cmd === '--help' || cmd === 'help' || cmd === '-h') {
    usage();
    return;
  }
  const p = policy();
  if (!p.enabled) emit({ ok: false, error: 'event_sourced_control_plane_disabled' }, 1);
  if (cmd === 'append') emit(appendEvent(args, p));
  if (cmd === 'set-authority') emit(cmdSetAuthority(args, p));
  if (cmd === 'rebuild') emit(cmdRebuild(args, p));
  if (cmd === 'reconcile') emit(cmdReconcile(args, p));
  if (cmd === 'replay') emit(replay(p));
  if (cmd === 'status') emit({
    ok: true,
    type: 'event_sourced_control_plane_status',
    latest: readJson(p.paths.latest_path, {}),
    authority_state: loadAuthorityState(p),
    materialized_view: readJson(p.paths.views_path, null),
    reconcile_latest: readJson(p.paths.reconcile_latest_path, null),
    rollback_latest: readJson(p.paths.rollback_latest_path, null),
    jetstream_latest: readJson(p.paths.jetstream_latest_path, null)
  });
  emit({ ok: false, error: 'unsupported_command', cmd }, 1);
}

main();
