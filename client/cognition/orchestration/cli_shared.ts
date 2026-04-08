#!/usr/bin/env node
'use strict';

const crypto = require('node:crypto');

function parseArgs(argv = []) {
  const positional = [];
  const flags = {};
  for (const raw of Array.isArray(argv) ? argv : []) {
    const token = String(raw || '').trim();
    if (!token) continue;
    if (token.startsWith('--')) {
      const body = token.slice(2);
      const eq = body.indexOf('=');
      if (eq >= 0) flags[body.slice(0, eq)] = body.slice(eq + 1);
      else flags[body] = '1';
      continue;
    }
    positional.push(token);
  }
  return { positional, flags };
}

function parseJson(raw, fallback, reasonCode) {
  if (raw == null || String(raw).trim() === '') return { ok: true, value: fallback };
  try {
    return { ok: true, value: JSON.parse(String(raw)) };
  } catch {
    return { ok: false, reason_code: reasonCode };
  }
}

function stableHash(input, length = 12) {
  return crypto.createHash('sha256').update(String(input || '')).digest('hex').slice(0, Math.max(4, Number(length) || 12));
}

function slug(raw, fallback = 'task', maxLen = 48) {
  const normalized = String(raw || '')
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, Math.max(4, Number(maxLen) || 48));
  return normalized || fallback;
}

function timestampToken(nowMs = Date.now()) {
  const d = new Date(nowMs);
  const year = String(d.getUTCFullYear());
  const month = String(d.getUTCMonth() + 1).padStart(2, '0');
  const day = String(d.getUTCDate()).padStart(2, '0');
  const hour = String(d.getUTCHours()).padStart(2, '0');
  const minute = String(d.getUTCMinutes()).padStart(2, '0');
  const second = String(d.getUTCSeconds()).padStart(2, '0');
  return `${year}${month}${day}${hour}${minute}${second}`;
}

function nonceToken(length = 6) {
  const width = Math.max(4, Number(length) || 6);
  return crypto.randomBytes(width).toString('hex').slice(0, width);
}

module.exports = {
  parseArgs,
  parseJson,
  stableHash,
  slug,
  timestampToken,
  nonceToken
};
