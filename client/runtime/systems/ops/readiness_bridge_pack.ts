#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const BRIDGE_PATH = 'client/runtime/systems/ops/readiness_bridge_pack.ts';
const KERNEL_ID = 'readiness-bridge-pack-kernel';
const DEFAULT_COMMAND = 'run';
const bridge = createOpsLaneBridge(__dirname, 'readiness_bridge_pack', 'readiness-bridge-pack-kernel', {
  // Keep readiness runs deterministic across workspace-bound test harnesses.
  preferLocalCore: true
});

function normalizeArgs(argv = []) {
  return Array.isArray(argv) ? argv.map((value) => String(value)) : [];
}

function writeBridgeOutput(out) {
  if (out.stdout) {
    const rawStdout = String(out.stdout);
    const trimmed = rawStdout.trim();
    let emitted = rawStdout;
    if (trimmed.startsWith('{') && trimmed.endsWith('}')) {
      try {
        const parsed = JSON.parse(trimmed);
        if (
          parsed &&
          parsed.type === 'ops_domain_conduit_runner_kernel' &&
          parsed.payload &&
          typeof parsed.payload === 'object'
        ) {
          let payload = parsed.payload;
          if (
            (!payload.type || typeof payload.type !== 'string') &&
            payload.payload &&
            typeof payload.payload === 'object'
          ) {
            payload = payload.payload;
          }
          emitted = `${JSON.stringify(payload)}\n`;
        }
      } catch (_) {
        emitted = rawStdout;
      }
    }
    process.stdout.write(emitted);
  }
  if (out.stderr) process.stderr.write(out.stderr);
  if (!out.stdout) {
    const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
    const payload = receipt && receipt.payload && typeof receipt.payload === 'object' ? receipt.payload : receipt;
    if (payload && typeof payload === 'object') {
      process.stdout.write(`${JSON.stringify(payload)}\n`);
    }
  }
}

function bridgeStatusCode(out) {
  const parsed = Number(out && out.status);
  return Number.isFinite(parsed) ? parsed : 1;
}

function main(argv = process.argv.slice(2)) {
  const normalizedArgs = normalizeArgs(argv);
  const out = bridge.run(normalizedArgs.length ? normalizedArgs : [DEFAULT_COMMAND]);
  writeBridgeOutput(out);
  return bridgeStatusCode(out);
}

function runPack(strict = true, options = {}) {
  const args = ['run', `--strict=${strict ? 1 : 0}`];
  if (options && typeof options === 'object' && options.policy) {
    args.push(`--policy=${String(options.policy)}`);
  }
  const out = bridge.run(args);
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  return receipt && receipt.payload && typeof receipt.payload === 'object'
    ? receipt.payload
    : receipt;
}

if (require.main === module) {
  process.exit(main(process.argv.slice(2)));
}

module.exports = {
  BRIDGE_PATH,
  KERNEL_ID,
  DEFAULT_COMMAND,
  main,
  runPack,
  normalizeArgs,
  writeBridgeOutput,
  bridgeStatusCode,
};
