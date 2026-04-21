#!/usr/bin/env node
'use strict';

const crypto = require('node:crypto');
const path = require('path');
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const DEFAULT_LANE = 'RUNTIME-LEGACY-ALIAS';
const MAX_LANE_LEN = 128;
const MAX_ARG_LEN = 512;
const MAX_ARGS = 64;
const bridge = createOpsLaneBridge(__dirname, 'legacy_alias_adapter', 'runtime-systems', {
  inheritStdio: true
});

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

function isUnsafeToken(token) {
  return (
    token.includes('..') ||
    token.includes('\0') ||
    token.startsWith('/') ||
    token.startsWith('\\')
  );
}

function normalizeLaneId(raw, fallback = DEFAULT_LANE) {
  const v = sanitizeArg(raw || '')
    .toUpperCase()
    .replace(/[^A-Z0-9_.-]+/g, '-')
    .replace(/^-+|-+$/g, '');
  return (v.slice(0, MAX_LANE_LEN) || fallback).slice(0, MAX_LANE_LEN);
}

function parseArgs(argv = []) {
  const args = Array.isArray(argv)
    ? argv
        .map((row) => sanitizeArg(row))
        .filter((row) => row && !isUnsafeToken(row))
        .slice(0, MAX_ARGS)
    : [];
  let laneId = '';
  let scriptPath = '';
  const passthrough = [];

  for (let i = 0; i < args.length; i += 1) {
    const token = String(args[i] || '');
    if (token.startsWith('--lane-id=')) {
      laneId = token.slice('--lane-id='.length).trim();
      continue;
    }
    if (token === '--lane-id') {
      laneId = String(args[i + 1] || '').trim();
      i += 1;
      continue;
    }
    if (token.startsWith('--script=')) {
      scriptPath = token.slice('--script='.length).trim();
      continue;
    }
    if (token === '--script') {
      scriptPath = String(args[i + 1] || '').trim();
      i += 1;
      continue;
    }
    passthrough.push(token);
  }

  return { laneId, scriptPath, passthrough };
}

function laneFromScript(scriptPath) {
  const raw = sanitizeArg(scriptPath || '');
  if (!raw) return '';
  const runtimeRoot = path.resolve(__dirname, '..', '..');
  const abs = path.resolve(raw);
  const rel = path.relative(runtimeRoot, abs).replace(/\\/g, '/').replace(/\.[^.]+$/, '');
  if (!rel || rel.startsWith('..')) return '';
  return normalizeLaneId(`RUNTIME-${rel}`, DEFAULT_LANE);
}

function laneFromAliasRel(aliasRel) {
  const rel = sanitizeArg(aliasRel || '')
    .replace(/\\/g, '/')
    .replace(/^\.\//, '')
    .replace(/\.[^.]+$/, '');
  if (!rel || rel.startsWith('..')) return '';
  return normalizeLaneId(`RUNTIME-${rel}`, DEFAULT_LANE);
}

function resolveLane(inputLaneId, scriptPath) {
  const lane = normalizeLaneId(String(inputLaneId || '').trim(), '');
  if (lane) return lane;
  const fromScript = laneFromScript(scriptPath);
  if (fromScript) return fromScript;
  return DEFAULT_LANE;
}

function normalizeBridgePayload(payload, laneId) {
  if (!payload || typeof payload !== 'object') {
    return payload;
  }
  const out = Object.assign({}, payload);
  if (typeof out.type !== 'string' || !out.type.trim()) {
    out.type = 'legacy_alias_adapter';
  }
  if (typeof out.lane_id !== 'string' || !out.lane_id.trim()) {
    out.lane_id = laneId;
  }
  out.lane_id = normalizeLaneId(out.lane_id, laneId);
  if (typeof out.receipt_hash !== 'string' || !out.receipt_hash.trim()) {
    out.receipt_hash = normalizeReceiptHash(out);
  }
  return out;
}

function runBridge(laneId, argv = []) {
  const args = Array.isArray(argv)
    ? argv
        .map((v) => sanitizeArg(v))
        .filter((row) => row && !isUnsafeToken(row))
        .slice(0, MAX_ARGS)
    : [];
  const out = bridge.run([`--lane-id=${laneId}`].concat(args));
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) {
    process.stdout.write(`${JSON.stringify(normalizeBridgePayload(out.payload, laneId))}\n`);
  } else if (!out || (!out.stdout && !out.stderr)) {
    const fallback = {
      ok: false,
      type: 'legacy_alias_adapter',
      error: 'bridge_no_output',
      lane_id: laneId,
      status: Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1
    };
    process.stdout.write(
      `${JSON.stringify(normalizeBridgePayload(fallback, laneId))}\n`
    );
  }
  return out;
}

function runLegacyAlias(spec = {}, argv = process.argv.slice(2)) {
  const explicitLane = normalizeLaneId(spec.lane_id || spec.laneId || '', '');
  const laneFromScriptPath = laneFromScript(spec.script || spec.scriptPath || '');
  const laneFromAlias = laneFromAliasRel(spec.alias_rel || spec.aliasRel || '');
  const laneId = normalizeLaneId(
    explicitLane || laneFromScriptPath || laneFromAlias || DEFAULT_LANE,
    DEFAULT_LANE
  );
  return runBridge(laneId, argv);
}

function run(argv = []) {
  const parsed = parseArgs(argv);
  const laneId = resolveLane(parsed.laneId, parsed.scriptPath);
  return runBridge(laneId, parsed.passthrough);
}

module.exports = {
  parseArgs,
  laneFromScript,
  laneFromAliasRel,
  normalizeReceiptHash,
  resolveLane,
  normalizeBridgePayload,
  runLegacyAlias,
  run
};

if (require.main === module) {
  const out = run(process.argv.slice(2));
  process.exit(Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1);
}
