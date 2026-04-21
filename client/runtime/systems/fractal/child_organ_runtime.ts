#!/usr/bin/env node
'use strict';
const crypto = require('node:crypto');
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const bridge = createOpsLaneBridge(__dirname, 'child_organ_runtime', 'child-organ-runtime', {
  inheritStdio: true
});

const MUTATION_COMMANDS = new Set(['plan', 'spawn']);
const ALLOWED_COMMANDS = new Set(['status', 'plan', 'spawn']);
const WRAPPER_TOKENS = new Set(['child-organ-runtime', 'child_organ_runtime']);
const COMMAND_ALIAS = Object.freeze({
  run: 'spawn',
  execute: 'spawn',
  prepare: 'plan',
  budget: 'plan',
  verify: 'status'
});
const MAX_ARGS = 64;
const MAX_ARG_LEN = 512;

function stableStringify(value) {
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map(stableStringify).join(',')}]`;
  }
  const keys = Object.keys(value).sort();
  return `{${keys
    .map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`)
    .join(',')}}`;
}

function normalizeReceiptHash(payload) {
  const clone = Object.assign({}, payload);
  delete clone.receipt_hash;
  return crypto.createHash('sha256').update(stableStringify(clone)).digest('hex');
}

function withReceiptHash(payload) {
  if (!payload || typeof payload !== 'object') {
    return payload;
  }
  const normalized = Object.assign({}, payload);
  if (typeof normalized.type !== 'string' || !normalized.type.trim()) {
    normalized.type = 'child_organ_runtime';
  }
  if (typeof normalized.lane !== 'string' || !normalized.lane.trim()) {
    normalized.lane = bridge.lane;
  }
  if (typeof normalized.receipt_hash === 'string' && normalized.receipt_hash.trim()) {
    return normalized;
  }
  return Object.assign({}, normalized, {
    receipt_hash: normalizeReceiptHash(normalized)
  });
}

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

function ensureMutationReceipt(result, command) {
  if (!result || !result.payload || typeof result.payload !== 'object') {
    return result;
  }
  if (!MUTATION_COMMANDS.has(command)) {
    return result;
  }
  return Object.assign({}, result, {
    payload: withReceiptHash(result.payload)
  });
}

function mapArgs(args = []) {
  const rows = (Array.isArray(args)
    ? args
        .map((v) => sanitizeArg(v))
        .filter((row) => row && !isUnsafeToken(row))
        .slice(0, MAX_ARGS)
    : []);
  while (rows.length && WRAPPER_TOKENS.has((rows[0] || '').toLowerCase())) {
    rows.shift();
  }
  if (!rows.length) return ['status'];
  const head = (rows[0] || '').toLowerCase();
  const tail = rows.slice(1);
  const mapped = COMMAND_ALIAS[head] || (ALLOWED_COMMANDS.has(head) ? head : 'status');
  if (mapped === 'status') {
    return ['status'];
  }
  return [mapped].concat(tail);
}

function run(args = process.argv.slice(2)) {
  const mapped = mapArgs(args);
  const command = mapped[0] || 'status';
  const out = ensureMutationReceipt(bridge.run(mapped), command);
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) {
    const payload =
      out.payload && (out.payload.ok === true || MUTATION_COMMANDS.has(command))
        ? withReceiptHash(out.payload)
        : out.payload;
    process.stdout.write(`${JSON.stringify(payload)}\n`);
  } else if (!out || (!out.stdout && !out.stderr)) {
    const fallback = {
      ok: false,
      type: 'child_organ_runtime',
      lane: bridge.lane,
      error: 'bridge_no_output',
      status: Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1
    };
    process.stdout.write(
      `${JSON.stringify(withReceiptHash(fallback))}\n`
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
  run,
  mapArgs,
  normalizeReceiptHash,
  ensureMutationReceipt
};
