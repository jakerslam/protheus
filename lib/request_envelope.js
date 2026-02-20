'use strict';

const crypto = require('crypto');

function normalizeLower(v) {
  return String(v || '').trim().toLowerCase();
}

function normalizeKeyId(v) {
  const raw = String(v || '').trim().toLowerCase();
  if (!raw) return '';
  return raw.replace(/[^a-z0-9._-]/g, '').slice(0, 40);
}

function secretKeyEnvVarName(kid) {
  const keyId = normalizeKeyId(kid);
  if (!keyId) return '';
  return `REQUEST_GATE_SECRET_${keyId.toUpperCase().replace(/[^A-Z0-9]/g, '_')}`;
}

function normalizeFiles(files) {
  const arr = Array.isArray(files) ? files : [];
  const out = [];
  const seen = new Set();
  for (const raw of arr) {
    const f = String(raw || '').trim().replace(/\\/g, '/');
    if (!f) continue;
    if (seen.has(f)) continue;
    seen.add(f);
    out.push(f);
  }
  out.sort((a, b) => a.localeCompare(b));
  return out;
}

function envelopePayload({ source, action, ts, nonce, files, kid }) {
  const src = normalizeLower(source) || 'local';
  const act = normalizeLower(action) || 'apply';
  const tsNum = Number.isFinite(Number(ts)) ? Math.floor(Number(ts)) : Math.floor(Date.now() / 1000);
  const nonceVal = String(nonce || '').trim() || crypto.randomBytes(12).toString('hex');
  const fileList = normalizeFiles(files);
  const keyId = normalizeKeyId(kid);
  return {
    source: src,
    action: act,
    kid: keyId,
    ts: tsNum,
    nonce: nonceVal,
    files: fileList
  };
}

function canonicalEnvelopeString(payload) {
  const p = envelopePayload(payload || {});
  return [
    'v1',
    `source=${p.source}`,
    `action=${p.action}`,
    `kid=${p.kid || ''}`,
    `ts=${p.ts}`,
    `nonce=${p.nonce}`,
    `files=${p.files.join(',')}`
  ].join('|');
}

function signEnvelope(payload, secret) {
  const key = String(secret || '');
  if (!key) return '';
  return crypto
    .createHmac('sha256', key)
    .update(canonicalEnvelopeString(payload), 'utf8')
    .digest('hex');
}

function safeEqualHex(a, b) {
  const ax = String(a || '').trim().toLowerCase();
  const bx = String(b || '').trim().toLowerCase();
  if (!/^[a-f0-9]{64}$/.test(ax) || !/^[a-f0-9]{64}$/.test(bx)) return false;
  return crypto.timingSafeEqual(Buffer.from(ax, 'hex'), Buffer.from(bx, 'hex'));
}

function verifyEnvelope({ source, action, ts, nonce, files, kid, signature, secret, maxSkewSec = 900, nowSec }) {
  const key = String(secret || '').trim();
  if (!key) return { ok: false, reason: 'secret_missing' };

  const payload = envelopePayload({ source, action, ts, nonce, files, kid });
  if (!Number.isFinite(Number(payload.ts)) || Number(payload.ts) <= 0) {
    return { ok: false, reason: 'timestamp_invalid' };
  }

  const now = Number.isFinite(Number(nowSec)) ? Math.floor(Number(nowSec)) : Math.floor(Date.now() / 1000);
  const maxSkew = Math.max(30, Number(maxSkewSec || 900));
  const skew = Math.abs(now - Number(payload.ts));
  if (skew > maxSkew) {
    return { ok: false, reason: 'timestamp_skew', skew_sec: skew, max_skew_sec: maxSkew };
  }

  const expected = signEnvelope(payload, key);
  const provided = String(signature || '').trim().toLowerCase();
  if (!safeEqualHex(expected, provided)) {
    return { ok: false, reason: 'signature_mismatch' };
  }

  return { ok: true, reason: 'ok', skew_sec: skew };
}

function stampGuardEnv(baseEnv, { source = 'local', action = 'apply', files = [], secret, ts, nonce, kid } = {}) {
  const env = { ...(baseEnv || {}) };
  const src = normalizeLower(source) || 'local';
  const act = normalizeLower(action) || 'apply';
  const keyId = normalizeKeyId(kid != null ? kid : env.REQUEST_KEY_ID);
  env.REQUEST_SOURCE = src;
  env.REQUEST_ACTION = act;
  if (keyId) env.REQUEST_KEY_ID = keyId;

  const keyFromKid = keyId ? String(env[secretKeyEnvVarName(keyId)] || '').trim() : '';
  const key = String(
    secret != null
      ? secret
      : keyFromKid || env.REQUEST_GATE_SECRET || ''
  ).trim();
  if (!key) return env;

  const payload = envelopePayload({ source: src, action: act, ts, nonce, files, kid: keyId });
  env.REQUEST_TS = String(payload.ts);
  env.REQUEST_NONCE = payload.nonce;
  env.REQUEST_SIG = signEnvelope(payload, key);
  return env;
}

function verifySignedEnvelopeFromEnv({ env = process.env, files = [], secret, maxSkewSec = 900, nowSec } = {}) {
  const e = env || {};
  const keyId = normalizeKeyId(e.REQUEST_KEY_ID);
  const keyFromKid = keyId ? String(e[secretKeyEnvVarName(keyId)] || '').trim() : '';
  const key = String(
    secret != null
      ? secret
      : keyFromKid || e.REQUEST_GATE_SECRET || ''
  ).trim();
  return verifyEnvelope({
    source: e.REQUEST_SOURCE,
    action: e.REQUEST_ACTION,
    kid: keyId,
    ts: e.REQUEST_TS,
    nonce: e.REQUEST_NONCE,
    files,
    signature: e.REQUEST_SIG,
    secret: key,
    maxSkewSec,
    nowSec
  });
}

module.exports = {
  envelopePayload,
  canonicalEnvelopeString,
  signEnvelope,
  verifyEnvelope,
  stampGuardEnv,
  verifySignedEnvelopeFromEnv,
  normalizeFiles,
  normalizeKeyId,
  secretKeyEnvVarName
};
