#!/usr/bin/env node
'use strict';
const crypto = require('node:crypto');
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const MUTATION_COMMANDS = new Set(['put', 'archive']);
const ALLOWED_COMMANDS = new Set(['put', 'get', 'status']);
const MAX_ARGS = 64;
const MAX_ARG_LEN = 512;

const bridge = createOpsLaneBridge(
  __dirname,
  'session_continuity_vault',
  'continuity-runtime',
  { inheritStdio: true }
);

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
  if (!MUTATION_COMMANDS.has(command)) {
    return result;
  }
  if (typeof result.payload.receipt_hash === 'string') {
    return result;
  }
  return Object.assign({}, result, {
    payload: Object.assign({}, result.payload, {
      receipt_hash: normalizeReceiptHash(result.payload)
    })
  });
}

function normalizeArgs(args = []) {
  const rows = Array.isArray(args)
    ? args.map((v) => sanitizeArg(v)).filter(Boolean).slice(0, MAX_ARGS)
    : [];
  if (!rows.length) {
    return ['status'];
  }

  let head = rows[0].toLowerCase();
  const tail = rows.slice(1);

  if (head === 'session-continuity-vault' || head === 'session_continuity_vault') {
    head = (tail[0] || '').toLowerCase();
    return normalizeArgs(tail);
  }
  if (!head) {
    return ['status'];
  }

  if (head === 'restore') {
    return ['get'].concat(tail);
  }
  if (head === 'archive') {
    return ['put'].concat(tail);
  }
  if (head === 'verify') {
    return ['status'].concat(tail);
  }

  if (!ALLOWED_COMMANDS.has(head)) {
    return ['status'].concat(tail);
  }

  return [head].concat(tail);
}

function run(args = process.argv.slice(2)) {
  const mapped = normalizeArgs(args);
  const command = mapped[0] || 'status';
  const out = ensureMutationReceipt(
    bridge.run(['session-continuity-vault'].concat(mapped)),
    command
  );
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  } else if (!out || (!out.stdout && !out.stderr)) {
    process.stdout.write(
      `${JSON.stringify({
        ok: false,
        type: 'session_continuity_vault',
        error: 'bridge_no_output',
        status: Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1
      })}\n`
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
  normalizeReceiptHash,
  ensureMutationReceipt
};
