'use strict';
export {};

// Layer ownership: core/layer1/security (authoritative)
// Thin TypeScript wrapper only.

const path = require('path');
const { runSecurityPlane } = require('../runtime/lib/security_plane_bridge.ts');

const REPO_ROOT = path.resolve(__dirname, '..', '..');
const STOP_PATH = path.join(REPO_ROOT, 'client', 'runtime', 'local', 'state', 'security', 'emergency_stop.json');
const VALID_SCOPES = new Set(['all', 'autonomy', 'routing', 'actuation', 'spine']);

function asString(v) {
  return String(v == null ? '' : v).trim();
}

function normalizeScopes(raw) {
  const src = Array.isArray(raw) ? raw : [raw];
  const out = [];
  for (const item of src) {
    for (const seg of String(item == null ? '' : item).split(',')) {
      const s = asString(seg).toLowerCase();
      if (!s || !VALID_SCOPES.has(s) || out.includes(s)) continue;
      out.push(s);
    }
  }
  if (!out.length) out.push('all');
  if (out.includes('all')) return ['all'];
  return out.sort((a, b) => a.localeCompare(b));
}

function extractPayload(out) {
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  return receipt && receipt.payload && typeof receipt.payload === 'object' ? receipt.payload : receipt;
}

function invoke(command, flags = {}, opts = {}) {
  const args = ['emergency-stop', command];
  for (const [key, value] of Object.entries(flags || {})) {
    if (value == null || value === '') continue;
    args.push(`--${key}=${value}`);
  }
  const out = runSecurityPlane('emergency-stop', [command, ...args.slice(2)]);
  const payload = extractPayload(out);
  if (out.status !== 0) {
    const message = payload && typeof payload.error === 'string'
      ? payload.error
      : (out && out.stderr ? String(out.stderr).trim() : `emergency_stop_${command}_failed`);
    if (opts.throwOnError === false) return { ok: false, error: message || `emergency_stop_${command}_failed` };
    throw new Error(message || `emergency_stop_${command}_failed`);
  }
  return payload && typeof payload === 'object' ? payload : { ok: false, error: `emergency_stop_${command}_bridge_failed` };
}

function getStopState() {
  const payload = invoke('status', {}, { throwOnError: false });
  const state = payload && payload.state && typeof payload.state === 'object'
    ? payload.state
    : { engaged: false, scopes: [], updated_at: null, reason: null, actor: null, approval_note: null };
  return {
    engaged: state.engaged === true,
    scopes: Array.isArray(state.scopes) ? state.scopes : [],
    updated_at: state.updated_at == null ? null : asString(state.updated_at),
    reason: state.reason == null ? null : asString(state.reason),
    actor: state.actor == null ? null : asString(state.actor),
    approval_note: state.approval_note == null ? null : asString(state.approval_note)
  };
}

function isEmergencyStopEngaged(scope) {
  const st = getStopState();
  if (!st.engaged) return { engaged: false, scope, state: st };
  const wanted = asString(scope).toLowerCase() || 'all';
  const hit = st.scopes.includes('all') || st.scopes.includes(wanted);
  return { engaged: hit, scope: wanted, state: st };
}

function engageEmergencyStop({ scopes, approval_note, actor, reason }) {
  const payload = invoke('engage', {
    scope: normalizeScopes(scopes).join(','),
    'approval-note': asString(approval_note).slice(0, 240),
    actor: asString(actor || process.env.USER || 'unknown').slice(0, 120),
    reason: asString(reason).slice(0, 240) || 'manual_emergency_stop'
  });
  return payload && payload.state && typeof payload.state === 'object' ? payload.state : getStopState();
}

function releaseEmergencyStop({ approval_note, actor, reason }) {
  const payload = invoke('release', {
    'approval-note': asString(approval_note).slice(0, 240),
    actor: asString(actor || process.env.USER || 'unknown').slice(0, 120),
    reason: asString(reason).slice(0, 240) || 'manual_release'
  });
  return payload && payload.state && typeof payload.state === 'object' ? payload.state : getStopState();
}

module.exports = {
  STOP_PATH,
  VALID_SCOPES,
  normalizeScopes,
  getStopState,
  isEmergencyStopEngaged,
  engageEmergencyStop,
  releaseEmergencyStop
};
