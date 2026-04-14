#!/usr/bin/env node
'use strict';
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

process.env.INFRING_OPS_ALLOW_PROCESS_FALLBACK = process.env.INFRING_OPS_ALLOW_PROCESS_FALLBACK || '0';
process.env.PROTHEUS_OPS_ALLOW_PROCESS_FALLBACK =
  process.env.PROTHEUS_OPS_ALLOW_PROCESS_FALLBACK || '0';

const SYSTEM_ID = 'SYSTEMS-TOOLS-CLI_SUGGESTION_ENGINE';
const bridge = createOpsLaneBridge(__dirname, 'cli_suggestion_engine', 'runtime-systems', {
  inheritStdio: true,
  preferLocalCore: true
});

function normalizeArgs(args) {
  if (!Array.isArray(args)) return [];
  return args.map((token) => String(token || '').trim()).filter(Boolean);
}

function ensureSystemId(args) {
  if (args.some((arg) => arg === '--system-id' || arg.startsWith('--system-id='))) {
    return args;
  }
  return [`--system-id=${SYSTEM_ID}`].concat(args);
}

function run(args = process.argv.slice(2)) {
  const normalized = ensureSystemId(normalizeArgs(args));
  const out = bridge.run(normalized);
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
  systemId: SYSTEM_ID,
  normalizeArgs,
  ensureSystemId,
  run
};
