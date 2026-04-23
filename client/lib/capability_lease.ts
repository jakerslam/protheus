'use strict';

// Layer ownership: core/layer1/security (authoritative)
// Thin TypeScript wrapper only.

const fs = require('fs');
const path = require('path');
const { createOpsLaneBridge } = require('../runtime/lib/rust_lane_bridge.ts');

const ROOT = path.resolve(__dirname, '..');
const LEASE_STATE_PATH = process.env.CAPABILITY_LEASE_STATE_PATH
  ? path.resolve(process.env.CAPABILITY_LEASE_STATE_PATH)
  : path.join(ROOT, 'local', 'state', 'security', 'capability_leases.json');
const LEASE_AUDIT_PATH = process.env.CAPABILITY_LEASE_AUDIT_PATH
  ? path.resolve(process.env.CAPABILITY_LEASE_AUDIT_PATH)
  : path.join(ROOT, 'local', 'state', 'security', 'capability_leases.jsonl');

process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'capability_lease', 'security-plane');

function text(value, maxLen = 240) {
  return String(value == null ? '' : value).trim().slice(0, maxLen);
}

function readJsonSafe(filePath, fallback) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function invoke(command, flags = {}, opts = {}) {
  const args = ['capability-lease', command];
  for (const [key, value] of Object.entries(flags || {})) {
    if (value == null) continue;
    args.push(`--${key}=${value}`);
  }
  const out = bridge.run(args);
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  const payload = receipt && receipt.payload && typeof receipt.payload === 'object'
    ? receipt.payload
    : receipt;
  if (out.status !== 0) {
    const message = payload && typeof payload.error === 'string'
      ? payload.error
      : (out && out.stderr ? String(out.stderr).trim() : `capability_lease_${command}_failed`);
    if (opts.throwOnError !== false) return { ok: false, error: message || `capability_lease_${command}_failed` };
    throw new Error(message || `capability_lease_${command}_failed`);
  }
  return payload && typeof payload === 'object' ? payload : { ok: false, error: `capability_lease_${command}_bridge_failed` };
}

function issueLease(opts = {}) {
  const input = opts && typeof opts === 'object' ? opts : {};
  return invoke('issue', {
    scope: text(input.scope, 180),
    target: text(input.target, 240) || undefined,
    'issued-by': text(input.issued_by || input.issuedBy || 'unknown', 120),
    reason: text(input.reason, 240) || undefined,
    'ttl-sec': Number.isFinite(Number(input.ttl_sec)) ? Math.floor(Number(input.ttl_sec)) : undefined
  });
}

function verifyLease(token, opts = {}) {
  const input = opts && typeof opts === 'object' ? opts : {};
  return invoke(input.consume === true ? 'consume' : 'verify', {
    token: text(token, 16384),
    scope: text(input.scope, 180) || undefined,
    target: text(input.target, 240) || undefined,
    reason: input.consume === true ? text(input.consume_reason || input.reason || 'consumed', 180) : undefined
  });
}

function loadLeaseState() {
  const raw = readJsonSafe(LEASE_STATE_PATH, null);
  if (!raw || typeof raw !== 'object') {
    return { version: '1.0', issued: {}, consumed: {} };
  }
  return {
    version: '1.0',
    issued: raw.issued && typeof raw.issued === 'object' ? raw.issued : {},
    consumed: raw.consumed && typeof raw.consumed === 'object' ? raw.consumed : {}
  };
}

module.exports = {
  issueLease,
  verifyLease,
  loadLeaseState,
  LEASE_STATE_PATH,
  LEASE_AUDIT_PATH
};
