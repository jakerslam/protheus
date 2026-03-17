#!/usr/bin/env node
'use strict';

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');
const { runOpsDomainCommand } = require('./spine_conduit_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'ops_domain_conduit_runner', 'ops-domain-conduit-runner-kernel');

function cleanText(v, maxLen = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function toBool(v, fallback = false) {
  const raw = cleanText(v, 32).toLowerCase();
  if (!raw) return fallback;
  if (['1', 'true', 'yes', 'on'].includes(raw)) return true;
  if (['0', 'false', 'no', 'off'].includes(raw)) return false;
  return fallback;
}

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function normalizeObject(value) {
  return value && typeof value === 'object' && !Array.isArray(value) ? { ...value } : {};
}

function invoke(command, payload = {}, opts = {}) {
  const out = bridge.run([
    command,
    `--payload-base64=${encodeBase64(JSON.stringify(normalizeObject(payload)))}`
  ]);
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  const payloadOut = receipt && receipt.payload && typeof receipt.payload === 'object'
    ? receipt.payload
    : receipt;
  if (out.status !== 0) {
    const message = payloadOut && typeof payloadOut.error === 'string'
      ? payloadOut.error
      : (out && out.stderr ? String(out.stderr).trim() : `ops_domain_conduit_runner_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `ops_domain_conduit_runner_kernel_${command}_failed`);
    return { ok: false, error: message || `ops_domain_conduit_runner_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `ops_domain_conduit_runner_kernel_${command}_bridge_failed`
      : `ops_domain_conduit_runner_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function parseArgs(argv) {
  return invoke('parse-argv', {
    argv: Array.isArray(argv) ? argv.map((value) => String(value)) : []
  }).parsed || { _: [] };
}

function buildPassArgs(parsedArgs) {
  return invoke('build-pass-args', {
    parsed: normalizeObject(parsedArgs)
  }).args || [];
}

function buildRunOptions(parsedArgs) {
  return invoke('build-run-options', {
    parsed: normalizeObject(parsedArgs)
  }).options || {
    runContext: null,
    skipRuntimeGate: true,
    stdioTimeoutMs: 120000,
    timeoutMs: 125000
  };
}

async function run(argv = process.argv.slice(2)) {
  const prepared = invoke('prepare-run', {
    argv: Array.isArray(argv) ? argv.map((value) => String(value)) : []
  });
  if (!prepared || !prepared.ok || !cleanText(prepared.domain || '', 120)) {
    return {
      status: 2,
      payload: {
        ok: false,
        type: 'ops_domain_conduit_bridge_error',
        reason: 'missing_domain',
        routed_via: 'conduit'
      }
    };
  }

  const result = await runOpsDomainCommand(
    cleanText(prepared.domain, 120),
    Array.isArray(prepared.args) ? prepared.args.map((value) => String(value)) : [],
    prepared.options && typeof prepared.options === 'object'
      ? prepared.options
      : buildRunOptions(parseArgs(argv))
  );
  return {
    status: Number.isFinite(result && result.status) ? Number(result.status) : 1,
    payload: result && result.payload
      ? result.payload
      : (result || {
        ok: false,
        type: 'ops_domain_conduit_bridge_error',
        reason: 'missing_result'
      }),
    result
  };
}

async function main() {
  const out = await run(process.argv.slice(2));
  const payload = out && out.payload
    ? out.payload
    : {
      ok: false,
      type: 'ops_domain_conduit_bridge_error',
      reason: 'missing_result',
      routed_via: 'conduit'
    };
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  process.exit(Number.isFinite(out && out.status) ? Number(out.status) : 1);
}

if (require.main === module) {
  main().catch((err) => {
    const out = {
      ok: false,
      type: 'ops_domain_conduit_bridge_error',
      reason: cleanText(err && err.message ? err.message : err, 220),
      routed_via: 'conduit'
    };
    process.stdout.write(`${JSON.stringify(out)}\n`);
    process.exit(1);
  });
}

module.exports = {
  cleanText,
  toBool,
  parseArgs,
  buildPassArgs,
  buildRunOptions,
  run,
  main
};
