#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');
const { normalizeOpsBridgeEnvAliases } = require('./queued_backlog_runtime.ts');

normalizeOpsBridgeEnvAliases();
process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_USE_PREBUILT =
  process.env.INFRING_OPS_USE_PREBUILT || process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS =
  process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'uid', 'uid-kernel');

function encodeBase64(value: unknown) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function invoke(command: string, payload: Record<string, unknown> = {}, opts: Record<string, unknown> = {}) {
  const out = bridge.run([
    command,
    `--payload-base64=${encodeBase64(JSON.stringify(payload || {}))}`
  ]);
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  const payloadOut = receipt && receipt.payload && typeof receipt.payload === 'object'
    ? receipt.payload
    : receipt;
  if (out.status !== 0) {
    const message = payloadOut && typeof payloadOut.error === 'string'
      ? payloadOut.error
      : (out && out.stderr ? String(out.stderr).trim() : `uid_kernel_${command}_failed`);
    if (opts && opts.throwOnError === false) return { ok: false, error: message || `uid_kernel_${command}_failed` };
    throw new Error(message || `uid_kernel_${command}_failed`);
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `uid_kernel_${command}_bridge_failed`
      : `uid_kernel_${command}_bridge_failed`;
    if (opts && opts.throwOnError === false) return { ok: false, error: message };
    throw new Error(message);
  }
  return payloadOut;
}

type UidOptions = {
  prefix?: string;
  length?: number;
};

function isAlnum(v: unknown): boolean {
  const out = invoke('is-alnum', { value: v });
  return out.result === true;
}

function stableUid(seed: unknown, opts: UidOptions = {}): string {
  const out = invoke('stable-uid', {
    seed: String(seed == null ? '' : seed),
    prefix: opts.prefix || '',
    length: opts.length
  });
  return String(out.uid || '');
}

function randomUid(opts: UidOptions = {}): string {
  const out = invoke('random-uid', {
    prefix: opts.prefix || '',
    length: opts.length
  });
  return String(out.uid || '');
}

module.exports = {
  isAlnum,
  stableUid,
  randomUid
};
