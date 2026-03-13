#!/usr/bin/env node
'use strict';
const crypto = require('node:crypto');
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const bridge = createOpsLaneBridge(__dirname, 'memory_federation_plane', 'memory-plane', {
  inheritStdio: true
});

const MUTATION_COMMANDS = new Set(['sync']);

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

function mapArgs(args = []) {
  const rows = Array.isArray(args) ? args.map((v) => String(v).trim()) : [];
  if (!rows.length) {
    return ['status'];
  }

  let head = rows[0].toLowerCase();
  const tail = rows.slice(1);

  if (head === 'memory-federation-plane' || head === 'memory_federation_plane') {
    head = (tail[0] || '').toLowerCase();
    return mapArgs(tail);
  }
  if (!head || head === 'status' || head === 'verify') {
    return ['status'].concat(tail);
  }
  if (head === 'push') {
    return ['sync'].concat(tail);
  }
  if (head === 'download' || head === 'collect' || head === 'snapshot') {
    return ['pull'].concat(tail);
  }
  return [head].concat(tail);
}

function run(args = process.argv.slice(2)) {
  const mapped = mapArgs(args);
  const command = mapped[0] || 'status';
  const out = ensureMutationReceipt(
    bridge.run(['memory-federation-plane'].concat(mapped)),
    command
  );
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
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
