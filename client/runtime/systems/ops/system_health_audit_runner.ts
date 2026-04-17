#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'system_health_audit_runner', 'system-health-audit-runner-kernel');

function normalizeArgs(argv = process.argv.slice(2)) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function resolvePassArgs(argv = process.argv.slice(2)) {
  const args = normalizeArgs(argv);
  if (args.length === 0) return ['run'];
  if ((args[0] || '').startsWith('--')) return ['run', ...args];
  return args;
}

function unwrapConduitEnvelopePayload(out) {
  const rawStdout = String(out && out.stdout ? out.stdout : '').trim();
  let parsed = null;
  if (rawStdout) {
    try {
      parsed = JSON.parse(rawStdout);
    } catch {}
  }
  if (!parsed && out && out.payload && typeof out.payload === 'object') {
    parsed = out.payload;
  }
  const nested =
    parsed &&
    parsed.type === 'ops_domain_conduit_runner_kernel' &&
    parsed.payload &&
    typeof parsed.payload === 'object' &&
    parsed.payload.payload &&
    typeof parsed.payload.payload === 'object'
      ? parsed.payload.payload
      : null;
  if (!nested) return out && out.stdout ? String(out.stdout) : '';
  return `${JSON.stringify(nested)}\n`;
}

function main(argv = process.argv.slice(2)) {
  const passArgs = resolvePassArgs(argv);
  const out = bridge.run(passArgs);
  const payloadText = unwrapConduitEnvelopePayload(out);
  if (payloadText) process.stdout.write(payloadText);
  if (out.stderr) process.stderr.write(out.stderr);
  return Number.isFinite(Number(out.status)) ? Number(out.status) : 1;
}

if (require.main === module) {
  process.exit(main(process.argv.slice(2)));
}

module.exports = {
  normalizeArgs,
  resolvePassArgs,
  unwrapConduitEnvelopePayload,
  main,
};
