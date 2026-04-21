#!/usr/bin/env node
'use strict';
const crypto = require('node:crypto');
const { createOpsLaneBridge } = require('../../../lib/rust_lane_bridge.ts');

const SYSTEM_ID = 'SYSTEMS-FRACTAL-WARDEN-COMPLEXITY_WARDEN_META_ORGAN';
const MAX_ARGS = 64;
const MAX_ARG_LEN = 512;
const MAX_STDOUT_BYTES = 512 * 1024;
const MAX_STDERR_BYTES = 256 * 1024;
const bridge = createOpsLaneBridge(__dirname, 'complexity_warden_meta_organ', 'runtime-systems', {
  inheritStdio: true
});

function sanitizeArg(value) {
  return String(value == null ? '' : value)
    .replace(/[\u200B\u200C\u200D\u2060\uFEFF]/g, '')
    .replace(/[\r\n\t]+/g, ' ')
    .replace(/[^\x20-\x7E]+/g, '')
    .trim()
    .slice(0, MAX_ARG_LEN);
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

function clipStream(value, maxBytes) {
  const text = typeof value === 'string' ? value : '';
  if (!text) return { text: '', truncated: false };
  const encoded = Buffer.from(text, 'utf8');
  if (encoded.length <= maxBytes) return { text, truncated: false };
  return { text: encoded.slice(0, maxBytes).toString('utf8'), truncated: true };
}

function buildPassthrough(args = []) {
  const source = Array.isArray(args) ? args : [];
  const out = [];
  let truncated = false;
  let dropped = 0;
  for (const item of source) {
    if (out.length >= MAX_ARGS) {
      truncated = true;
      break;
    }
    const clean = sanitizeArg(item);
    if (!clean) {
      dropped += 1;
      continue;
    }
    out.push(clean);
  }
  return {
    args: out,
    arg_count: out.length,
    arg_truncated: truncated,
    dropped_empty_args: dropped
  };
}

function run(args = process.argv.slice(2)) {
  const passthrough = buildPassthrough(args);
  const out = bridge.run([`--system-id=${SYSTEM_ID}`].concat(passthrough.args));
  const clippedStdout = clipStream(out && out.stdout, MAX_STDOUT_BYTES);
  const clippedStderr = clipStream(out && out.stderr, MAX_STDERR_BYTES);
  if (clippedStdout.text) process.stdout.write(clippedStdout.text);
  if (clippedStderr.text) process.stderr.write(clippedStderr.text);
  if (out && out.payload && !clippedStdout.text) {
    process.stdout.write(
      `${JSON.stringify(withReceiptHash(Object.assign({
        lane: bridge.lane,
        bridge_contract: {
          max_args: MAX_ARGS,
          max_arg_len: MAX_ARG_LEN,
          arg_count: passthrough.arg_count,
          arg_truncated: passthrough.arg_truncated,
          dropped_empty_args: passthrough.dropped_empty_args,
          stdout_truncated: clippedStdout.truncated,
          stderr_truncated: clippedStderr.truncated
        }
      }, out.payload)))}\n`
    );
  } else if (!out || (!clippedStdout.text && !clippedStderr.text)) {
    const fallback = {
      ok: false,
      type: 'complexity_warden_meta_organ',
      lane: bridge.lane,
      error: 'bridge_no_output',
      status: Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1,
      bridge_contract: {
        max_args: MAX_ARGS,
        max_arg_len: MAX_ARG_LEN,
        arg_count: passthrough.arg_count,
        arg_truncated: passthrough.arg_truncated,
        dropped_empty_args: passthrough.dropped_empty_args,
        stdout_truncated: clippedStdout.truncated,
        stderr_truncated: clippedStderr.truncated
      }
    };
    process.stdout.write(
      `${JSON.stringify(withReceiptHash(fallback))}\n`
    );
  }
  return out;
}

if (require.main === module) {
  const out = run(process.argv.slice(2));
  process.exit(Number.isFinite(Number(out && out.status)) ? Number(out && out.status) : 1);
}

module.exports = {
  lane: bridge.lane,
  systemId: SYSTEM_ID,
  run,
  normalizeReceiptHash
};
