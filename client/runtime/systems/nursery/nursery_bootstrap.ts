#!/usr/bin/env node
'use strict';
const crypto = require('node:crypto');
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const SYSTEM_ID = 'V6-INFRING-DETACH-001.1';
const MAX_ARGS = 64;
const MAX_ARG_LEN = 512;
const bridge = createOpsLaneBridge(__dirname, 'nursery_bootstrap', 'runtime-systems', {
  inheritStdio: true,
  preferLocalCore: true
});

function sanitizeArg(value) {
  return String(value == null ? '' : value)
    .replace(/[\u200B\u200C\u200D\u2060\uFEFF]/g, '')
    .replace(/[\r\n\t]+/g, ' ')
    .replace(/[^\x20-\x7E]+/g, '')
    .trim()
    .slice(0, MAX_ARG_LEN);
}

function isUnsafeToken(token) {
  return token.includes('..') || token.includes('\0') || token.startsWith('/') || token.startsWith('\\');
}

function stableStringify(value) {
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map((item) => stableStringify(item)).join(',')}]`;
  }
  const keys = Object.keys(value).sort();
  return `{${keys.map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`).join(',')}}`;
}

function normalizeReceiptHash(payload) {
  const clone = Object.assign({}, payload);
  delete clone.receipt_hash;
  return crypto.createHash('sha256').update(stableStringify(clone)).digest('hex');
}

function withReceiptHash(payload) {
  if (!payload || typeof payload !== 'object') return payload;
  if (typeof payload.receipt_hash === 'string' && payload.receipt_hash.trim()) return payload;
  return Object.assign({}, payload, { receipt_hash: normalizeReceiptHash(payload) });
}

function normalizeBridgePayload(payload) {
  if (!payload || typeof payload !== 'object') return payload;
  const normalized = Object.assign({}, payload);
  if (typeof normalized.type !== 'string' || !normalized.type.trim()) {
    normalized.type = 'nursery_bootstrap';
  }
  if (typeof normalized.lane !== 'string' || !normalized.lane.trim()) {
    normalized.lane = bridge.lane;
  }
  return withReceiptHash(normalized);
}

function defaultPayloadArg() {
  const sourceRoot = sanitizeArg(process.env.INFRING_INFRING_SOURCE_ROOT || '..') || '..';
  return `--payload-json=${JSON.stringify({ source_root: sourceRoot })}`;
}

function run(args = process.argv.slice(2)) {
  const passthrough = Array.isArray(args)
    ? args
        .map((arg) => sanitizeArg(arg))
        .filter((row) => row && !isUnsafeToken(row))
        .slice(0, MAX_ARGS)
    : [];
  if (!passthrough.some((row) => String(row).startsWith('--strict='))) passthrough.push('--strict=1');
  if (!passthrough.some((row) => String(row).startsWith('--apply='))) passthrough.push('--apply=1');
  if (!passthrough.some((row) => String(row).startsWith('--payload-json='))) {
    passthrough.push(defaultPayloadArg());
  }
  const out = bridge.run(['run', `--system-id=${SYSTEM_ID}`].concat(passthrough));
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) {
    process.stdout.write(
      `${JSON.stringify(normalizeBridgePayload(out.payload))}\n`
    );
  } else if (!out || (!out.stdout && !out.stderr)) {
    const fallback = {
      ok: false,
      type: 'nursery_bootstrap',
      lane: bridge.lane,
      error: 'bridge_no_output',
      status: Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1
    };
    process.stdout.write(
      `${JSON.stringify(normalizeBridgePayload(fallback))}\n`
    );
  }
  return out;
}

if (require.main === module) {
  const out = run(process.argv.slice(2));
  process.exit(Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1);
}

module.exports = {
  lane: bridge.lane,
  systemId: SYSTEM_ID,
  run,
  normalizeReceiptHash
};
