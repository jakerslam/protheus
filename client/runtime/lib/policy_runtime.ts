'use strict';

export {};

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const path = require('path') as typeof import('path');
const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');
const {
  ROOT,
  resolvePath
} = require('./queued_backlog_runtime.ts');

type AnyObj = Record<string, any>;

type LoadPolicyRuntimeOptions = {
  policyPath: unknown,
  defaults: AnyObj,
  normalize?: (ctx: {
    raw: AnyObj,
    defaults: AnyObj,
    merged: AnyObj,
    policyPath: string,
    root: string
  }) => AnyObj
};

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'policy_runtime', 'policy-runtime-kernel');

function encodeBase64(value: unknown) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function invoke(command: string, payload: Record<string, unknown> = {}, opts: { throwOnError?: boolean } = {}) {
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
      : (out && out.stderr ? String(out.stderr).trim() : `policy_runtime_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `policy_runtime_kernel_${command}_failed`);
    return { ok: false, error: message || `policy_runtime_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `policy_runtime_kernel_${command}_bridge_failed`
      : `policy_runtime_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function isPlainObject(v: unknown): v is AnyObj {
  return !!v && typeof v === 'object' && !Array.isArray(v);
}

function clone(value: unknown): unknown {
  if (Array.isArray(value)) return value.map((row) => clone(row));
  if (isPlainObject(value)) {
    const out: AnyObj = {};
    for (const [k, v] of Object.entries(value)) out[k] = clone(v);
    return out;
  }
  return value;
}

function deepMerge(baseValue: unknown, overrideValue: unknown): unknown {
  const out = invoke('deep-merge', {
    base: baseValue,
    override: overrideValue === undefined ? null : overrideValue
  });
  return clone(out.value);
}

function resolvePolicyPath(rawPath: unknown) {
  const txt = String(rawPath == null ? '' : rawPath).trim();
  if (!txt) return '';
  const out = invoke('resolve-policy-path', {
    root_dir: ROOT,
    policy_path: txt
  });
  return String(out.policy_path || '');
}

function loadPolicyRuntime(opts: LoadPolicyRuntimeOptions) {
  const defaults = isPlainObject(opts && opts.defaults) ? opts.defaults : {};
  const out = invoke('load-policy-runtime', {
    root_dir: ROOT,
    policy_path: opts && opts.policyPath,
    defaults
  });
  const runtime = out.runtime && typeof out.runtime === 'object' ? out.runtime : {};
  const raw = runtime.raw && typeof runtime.raw === 'object' ? runtime.raw : {};
  const merged = runtime.merged && typeof runtime.merged === 'object' ? runtime.merged : {};
  const normalize = opts && typeof opts.normalize === 'function' ? opts.normalize : null;
  const policy = normalize
    ? normalize({ raw, defaults, merged, policyPath: String(runtime.policy_path || ''), root: ROOT })
    : merged;
  return {
    policy,
    raw,
    defaults,
    merged,
    policy_path: String(runtime.policy_path || '')
  };
}

function resolvePolicyValuePath(raw: unknown, fallbackRel: string) {
  return resolvePath(raw || fallbackRel, fallbackRel);
}

module.exports = {
  ROOT,
  loadPolicyRuntime,
  resolvePolicyPath,
  resolvePolicyValuePath,
  deepMerge
};
