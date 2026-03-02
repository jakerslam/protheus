#!/usr/bin/env node
'use strict';
export {};

/**
 * BL-016
 * Cross-device active-state continuity layer with lease, checkpoint and replay.
 *
 * Usage:
 *   node systems/continuity/active_state_continuity_layer.js lease-acquire --device=<id> [--ttl-sec=<n>] [--strict=1|0]
 *   node systems/continuity/active_state_continuity_layer.js checkpoint --device=<id> --state-json="{...}" [--strict=1|0]
 *   node systems/continuity/active_state_continuity_layer.js replay --to-device=<id>
 *   node systems/continuity/active_state_continuity_layer.js status
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

type AnyObj = Record<string, any>;

const ROOT = process.env.ACTIVE_STATE_CONTINUITY_ROOT
  ? path.resolve(process.env.ACTIVE_STATE_CONTINUITY_ROOT)
  : path.resolve(__dirname, '..', '..');

const DEFAULT_POLICY_PATH = process.env.ACTIVE_STATE_CONTINUITY_POLICY_PATH
  ? path.resolve(process.env.ACTIVE_STATE_CONTINUITY_POLICY_PATH)
  : path.join(ROOT, 'config', 'active_state_continuity_policy.json');

function nowIso() { return new Date().toISOString(); }
function cleanText(v: unknown, maxLen = 360) { return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen); }
function parseArgs(argv: string[]) {
  const out: AnyObj = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const tok = String(argv[i] || '');
    if (!tok.startsWith('--')) { out._.push(tok); continue; }
    const eq = tok.indexOf('=');
    if (eq >= 0) { out[tok.slice(2, eq)] = tok.slice(eq + 1); continue; }
    const key = tok.slice(2);
    const next = argv[i + 1];
    if (next != null && !String(next).startsWith('--')) { out[key] = String(next); i += 1; continue; }
    out[key] = true;
  }
  return out;
}
function toBool(v: unknown, fallback = false) {
  if (v == null) return fallback;
  const raw = String(v).trim().toLowerCase();
  if (['1', 'true', 'yes', 'on'].includes(raw)) return true;
  if (['0', 'false', 'no', 'off'].includes(raw)) return false;
  return fallback;
}
function clampInt(v: unknown, lo: number, hi: number, fallback: number) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  if (n < lo) return lo;
  if (n > hi) return hi;
  return Math.trunc(n);
}
function ensureDir(dirPath: string) { fs.mkdirSync(dirPath, { recursive: true }); }
function readJson(filePath: string, fallback: any = null) {
  try { if (!fs.existsSync(filePath)) return fallback; const parsed = JSON.parse(fs.readFileSync(filePath, 'utf8')); return parsed == null ? fallback : parsed; } catch { return fallback; }
}
function writeJsonAtomic(filePath: string, value: AnyObj) {
  ensureDir(path.dirname(filePath)); const tmp = `${filePath}.tmp-${Date.now()}-${process.pid}`;
  fs.writeFileSync(tmp, `${JSON.stringify(value, null, 2)}\n`, 'utf8'); fs.renameSync(tmp, filePath);
}
function appendJsonl(filePath: string, row: AnyObj) { ensureDir(path.dirname(filePath)); fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`, 'utf8'); }
function resolvePath(raw: unknown, fallbackRel: string) { const txt = cleanText(raw, 520); if (!txt) return path.join(ROOT, fallbackRel); return path.isAbsolute(txt) ? txt : path.join(ROOT, txt); }
function rel(filePath: string) { return path.relative(ROOT, filePath).replace(/\\/g, '/'); }
function parseJsonArg(raw: unknown, fallback: any = null) { const txt = cleanText(raw, 100000); if (!txt) return fallback; try { return JSON.parse(txt); } catch { return fallback; } }

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    lease_ttl_sec: 300,
    redact_keys: ['token', 'secret', 'api_key', 'password', 'credential'],
    outputs: {
      state_path: 'state/continuity/active_state_continuity/state.json',
      latest_path: 'state/continuity/active_state_continuity/latest.json',
      history_path: 'state/continuity/active_state_continuity/history.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const base = defaultPolicy();
  const raw = readJson(policyPath, {});
  const outputs = raw.outputs && typeof raw.outputs === 'object' ? raw.outputs : {};
  const redactKeys = Array.isArray(raw.redact_keys)
    ? raw.redact_keys.map((x: unknown) => cleanText(x, 80).toLowerCase()).filter(Boolean)
    : base.redact_keys;
  return {
    version: cleanText(raw.version || base.version, 40) || base.version,
    enabled: raw.enabled !== false,
    lease_ttl_sec: clampInt(raw.lease_ttl_sec, 30, 24 * 60 * 60, base.lease_ttl_sec),
    redact_keys: Array.from(new Set(redactKeys)),
    outputs: {
      state_path: resolvePath(outputs.state_path, base.outputs.state_path),
      latest_path: resolvePath(outputs.latest_path, base.outputs.latest_path),
      history_path: resolvePath(outputs.history_path, base.outputs.history_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function loadState(statePath: string) {
  const raw = readJson(statePath, {
    version: 1,
    updated_at: null,
    lease: null,
    checkpoints: [],
    replay_cursor: 0
  });
  if (!raw || typeof raw !== 'object') return { version: 1, updated_at: null, lease: null, checkpoints: [], replay_cursor: 0 };
  if (!Array.isArray(raw.checkpoints)) raw.checkpoints = [];
  return raw;
}

function isLeaseActive(lease: AnyObj) {
  if (!lease || typeof lease !== 'object') return false;
  const untilMs = Date.parse(String(lease.expires_at || ''));
  return Number.isFinite(untilMs) && untilMs > Date.now();
}

function redactSecrets(value: any, keys: string[]): any {
  if (Array.isArray(value)) return value.map((v) => redactSecrets(v, keys));
  if (!value || typeof value !== 'object') return value;
  const out: AnyObj = {};
  for (const [k, v] of Object.entries(value)) {
    const lower = String(k || '').toLowerCase();
    if (keys.some((needle) => lower.includes(needle))) {
      out[k] = '[REDACTED]';
    } else {
      out[k] = redactSecrets(v, keys);
    }
  }
  return out;
}

function checkpointDigest(payload: AnyObj) {
  return crypto.createHash('sha256').update(JSON.stringify(payload), 'utf8').digest('hex').slice(0, 16);
}

function cmdLeaseAcquire(args: AnyObj) {
  const strict = toBool(args.strict, true);
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  if (!policy.enabled) return { ok: true, strict, result: 'disabled_by_policy', policy_path: rel(policy.policy_path) };

  const device = cleanText(args.device, 120);
  if (!device) return { ok: false, error: 'missing_device' };
  const state = loadState(policy.outputs.state_path);

  if (isLeaseActive(state.lease) && String(state.lease.device || '') !== device) {
    return {
      ok: false,
      type: 'active_state_continuity_lease',
      error: 'lease_held_by_other_device',
      holder: state.lease,
      policy_path: rel(policy.policy_path)
    };
  }

  const ttlSec = clampInt(args['ttl-sec'] || args.ttl_sec, 30, 24 * 60 * 60, policy.lease_ttl_sec);
  const lease = {
    lease_id: `lease_${checkpointDigest({ device, ts: nowIso() })}`,
    device,
    acquired_at: nowIso(),
    expires_at: new Date(Date.now() + (ttlSec * 1000)).toISOString(),
    ttl_sec: ttlSec
  };

  state.lease = lease;
  state.updated_at = nowIso();
  writeJsonAtomic(policy.outputs.state_path, state);

  const out = {
    ok: true,
    ts: nowIso(),
    type: 'active_state_continuity_lease',
    strict,
    lease,
    policy_path: rel(policy.policy_path)
  };
  writeJsonAtomic(policy.outputs.latest_path, out);
  appendJsonl(policy.outputs.history_path, { ts: out.ts, type: out.type, device, lease_id: lease.lease_id, ok: true });
  return out;
}

function cmdCheckpoint(args: AnyObj) {
  const strict = toBool(args.strict, true);
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  if (!policy.enabled) return { ok: true, strict, result: 'disabled_by_policy', policy_path: rel(policy.policy_path) };

  const device = cleanText(args.device, 120);
  if (!device) return { ok: false, error: 'missing_device' };
  const state = loadState(policy.outputs.state_path);
  if (!isLeaseActive(state.lease) || String(state.lease.device || '') !== device) {
    return { ok: false, type: 'active_state_continuity_checkpoint', error: 'lease_not_held_by_device', lease: state.lease || null };
  }

  const input = parseJsonArg(args['state-json'] || args.state_json || '', null);
  if (!input || typeof input !== 'object') return { ok: false, error: 'invalid_state_json' };
  const payload = redactSecrets(input, policy.redact_keys || []);

  const checkpoint = {
    checkpoint_id: `ckpt_${checkpointDigest({ device, payload, ts: nowIso() })}`,
    device,
    ts: nowIso(),
    payload,
    digest: checkpointDigest(payload)
  };
  state.checkpoints.push(checkpoint);
  state.checkpoints = state.checkpoints.slice(-64);
  state.updated_at = nowIso();
  writeJsonAtomic(policy.outputs.state_path, state);

  const out = {
    ok: true,
    ts: nowIso(),
    type: 'active_state_continuity_checkpoint',
    strict,
    checkpoint: {
      checkpoint_id: checkpoint.checkpoint_id,
      device: checkpoint.device,
      digest: checkpoint.digest
    },
    policy_path: rel(policy.policy_path)
  };
  writeJsonAtomic(policy.outputs.latest_path, out);
  appendJsonl(policy.outputs.history_path, { ts: out.ts, type: out.type, checkpoint_id: checkpoint.checkpoint_id, device, ok: true });
  return out;
}

function cmdReplay(args: AnyObj) {
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  const toDevice = cleanText(args['to-device'] || args.to_device, 120);
  if (!toDevice) return { ok: false, error: 'missing_to_device' };

  const state = loadState(policy.outputs.state_path);
  const latest = state.checkpoints.length ? state.checkpoints[state.checkpoints.length - 1] : null;
  if (!latest) return { ok: false, type: 'active_state_continuity_replay', error: 'no_checkpoint_available' };

  state.replay_cursor = Number(state.replay_cursor || 0) + 1;
  state.updated_at = nowIso();
  writeJsonAtomic(policy.outputs.state_path, state);

  const out = {
    ok: true,
    ts: nowIso(),
    type: 'active_state_continuity_replay',
    to_device: toDevice,
    replay_cursor: state.replay_cursor,
    checkpoint: {
      checkpoint_id: latest.checkpoint_id,
      from_device: latest.device,
      digest: latest.digest,
      payload: latest.payload
    },
    policy_path: rel(policy.policy_path)
  };
  writeJsonAtomic(policy.outputs.latest_path, out);
  appendJsonl(policy.outputs.history_path, { ts: out.ts, type: out.type, to_device: toDevice, checkpoint_id: latest.checkpoint_id, replay_cursor: state.replay_cursor, ok: true });
  return out;
}

function cmdStatus(args: AnyObj) {
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  const state = loadState(policy.outputs.state_path);
  return {
    ok: true,
    ts: nowIso(),
    type: 'active_state_continuity_status',
    policy_path: rel(policy.policy_path),
    state_path: rel(policy.outputs.state_path),
    lease_active: isLeaseActive(state.lease),
    lease: state.lease,
    checkpoint_count: Array.isArray(state.checkpoints) ? state.checkpoints.length : 0,
    replay_cursor: Number(state.replay_cursor || 0),
    latest: readJson(policy.outputs.latest_path, null)
  };
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/continuity/active_state_continuity_layer.js lease-acquire --device=<id> [--ttl-sec=<n>]');
  console.log('  node systems/continuity/active_state_continuity_layer.js checkpoint --device=<id> --state-json="{...}"');
  console.log('  node systems/continuity/active_state_continuity_layer.js replay --to-device=<id>');
  console.log('  node systems/continuity/active_state_continuity_layer.js status');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = String(args._[0] || 'status').toLowerCase();
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') { usage(); return; }
  try {
    const payload = cmd === 'lease-acquire' ? cmdLeaseAcquire(args)
      : cmd === 'checkpoint' ? cmdCheckpoint(args)
      : cmd === 'replay' ? cmdReplay(args)
      : cmd === 'status' ? cmdStatus(args)
      : { ok: false, error: `unknown_command:${cmd}` };
    process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
    if (payload.ok === false && toBool(args.strict, true)) process.exit(1);
    if (payload.ok === false) process.exit(1);
  } catch (err) {
    process.stdout.write(`${JSON.stringify({ ok: false, error: cleanText((err as AnyObj)?.message || err || 'active_state_continuity_layer_failed', 260) })}\n`);
    process.exit(1);
  }
}

if (require.main === module) main();

module.exports = { loadPolicy, cmdLeaseAcquire, cmdCheckpoint, cmdReplay, cmdStatus, redactSecrets };
