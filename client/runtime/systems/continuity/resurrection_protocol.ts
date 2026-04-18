#!/usr/bin/env node
'use strict';
const crypto = require('node:crypto');
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const bridge = createOpsLaneBridge(
  __dirname,
  'resurrection_protocol',
  'continuity-runtime',
  { inheritStdio: true }
);

/**
 * @typedef {'checkpoint' | 'restore' | 'status' | 'verify' | 'run' | 'build' | 'bundle'}
 * ResurrectionSurfaceCommand
 */

/**
 * @typedef {{
 *   ok: boolean,
 *   type: string,
 *   lane: string,
 *   session_id?: string,
 *   apply?: boolean,
 *   receipt_hash?: string,
 *   claim_evidence?: Array<{ id: string, claim: string, evidence: Record<string, unknown> }>,
 * }} ContinuityProtocolPayload
 */

const MUTATION_COMMANDS = new Set(['checkpoint']);
const ALLOWED_COMMANDS = new Set(['checkpoint', 'restore', 'status']);
const WRAPPER_TOKENS = new Set(['resurrection-protocol', 'resurrection_protocol']);
const COMMAND_ALIAS = Object.freeze({
  run: 'checkpoint',
  build: 'checkpoint',
  bundle: 'checkpoint',
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
  if (typeof payload.receipt_hash === 'string' && payload.receipt_hash.trim()) {
    return payload;
  }
  return Object.assign({}, payload, {
    receipt_hash: normalizeReceiptHash(payload)
  });
}

function requiresReceipt(command, payload) {
  if (MUTATION_COMMANDS.has(command)) {
    return true;
  }
  return Boolean(payload && typeof payload === 'object' && payload.apply === true);
}

function sanitizeArg(value) {
  return String(value == null ? '' : value)
    .replace(/[\u200B\u200C\u200D\u2060\uFEFF]/g, '')
    .replace(/[\r\n\t]+/g, ' ')
    .replace(/[^\x20-\x7E]+/g, '')
    .trim()
    .slice(0, MAX_ARG_LEN);
}

function ensureMutationReceipt(result, command) {
  if (!result || !result.payload || typeof result.payload !== 'object') {
    return result;
  }
  if (!requiresReceipt(command, result.payload)) {
    return result;
  }
  return Object.assign({}, result, {
    payload: withReceiptHash(result.payload)
  });
}

function normalizeArgs(args = []) {
  const rows = (Array.isArray(args)
    ? args.map((v) => sanitizeArg(v)).filter(Boolean).slice(0, MAX_ARGS)
    : []);
  while (rows.length && WRAPPER_TOKENS.has((rows[0] || '').toLowerCase())) {
    rows.shift();
  }

  if (!rows.length) {
    return ['status'];
  }
  const head = (rows[0] || '').toLowerCase();
  const tail = rows.slice(1);
  const mapped = COMMAND_ALIAS[head] || (ALLOWED_COMMANDS.has(head) ? head : 'status');
  if (mapped === 'status') {
    return ['status'];
  }
  return [mapped].concat(tail);
}

function run(args = process.argv.slice(2)) {
  const mapped = normalizeArgs(args);
  const command = mapped[0] || 'status';
  const out = ensureMutationReceipt(
    bridge.run(['resurrection-protocol'].concat(mapped)),
    command
  );
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) {
    const payload =
      out.payload && (out.payload.ok === true || requiresReceipt(command, out.payload))
        ? withReceiptHash(out.payload)
        : out.payload;
    process.stdout.write(`${JSON.stringify(payload)}\n`);
  } else if (!out || (!out.stdout && !out.stderr)) {
    const fallback = {
      ok: false,
      type: 'resurrection_protocol',
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
  normalizeArgs,
  ensureMutationReceipt,
  normalizeReceiptHash
};
