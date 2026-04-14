#!/usr/bin/env node
'use strict';
const crypto = require('node:crypto');
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');
const { commandNameFromArgs, validateMemoryPolicy, guardFailureResult } = require('./policy_validator.ts');
const { validateSessionIsolation, sessionFailureResult } = require('./session_isolation.ts');

const SYSTEM_ID = 'SYSTEMS-MEMORY-CAUSAL_TEMPORAL_GRAPH';
const bridge = createOpsLaneBridge(__dirname, 'causal_temporal_graph', 'memory-plane', {
  inheritStdio: true
});

const MUTATION_COMMANDS = new Set(['record']);

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

  if (head === 'causal-temporal-graph' || head === 'causal_temporal_graph') {
    head = (tail[0] || '').toLowerCase();
    return mapArgs(tail);
  }
  if (!head || head === 'status' || head === 'verify') {
    return ['status'].concat(tail);
  }
  if (head === 'build' || head === 'record-legacy') {
    return [
      'record',
      '--event-id=build-latest',
      '--summary=legacy_build_alias',
      '--actor=system',
      '--apply=0'
    ].concat(tail);
  }
  if (head === 'query' || head === 'lookup') {
    return ['blame', '--event-id=build-latest'].concat(tail);
  }
  return [head].concat(tail);
}

function emitBridgeResult(out) {
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  }
  return Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1;
}

function run(args = process.argv.slice(2)) {
  const mapped = mapArgs(args);
  const command = commandNameFromArgs(mapped, 'status');
  const policy = validateMemoryPolicy(mapped, { command, lane: SYSTEM_ID });
  let out;
  if (!policy.ok) {
    out = guardFailureResult(policy, { command, system_id: SYSTEM_ID });
  } else {
    const isolation = validateSessionIsolation(mapped, {
      command,
      lane: SYSTEM_ID
    });
    if (!isolation.ok) {
      out = sessionFailureResult(isolation, { command, system_id: SYSTEM_ID });
    } else {
      out = ensureMutationReceipt(
        bridge.run(['causal-temporal-graph'].concat(mapped).concat([`--system-id=${SYSTEM_ID}`])),
        command
      );
    }
  }
  return out;
}

if (require.main === module) {
  process.exit(emitBridgeResult(run(process.argv.slice(2))));
}

module.exports = {
  lane: bridge.lane,
  systemId: SYSTEM_ID,
  run,
  mapArgs,
  emitBridgeResult,
  normalizeReceiptHash,
  ensureMutationReceipt
};
