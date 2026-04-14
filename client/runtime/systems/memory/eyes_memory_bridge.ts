#!/usr/bin/env node
'use strict';
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');
const { commandNameFromArgs, validateMemoryPolicy, guardFailureResult } = require('./policy_validator.ts');
const { validateSessionIsolation, sessionFailureResult } = require('./session_isolation.ts');

const BRIDGE_PATH = 'client/runtime/systems/memory/eyes_memory_bridge.ts';
const SYSTEM_ID = 'SYSTEMS-MEMORY-EYES_MEMORY_BRIDGE';
const LANE_DOMAIN = 'runtime-systems';
const DEFAULT_COMMAND = 'status';
const bridge = createOpsLaneBridge(__dirname, 'eyes_memory_bridge', 'runtime-systems', {
  inheritStdio: true
});

function normalizeArgs(args = []) {
  return Array.isArray(args) ? args.map((row) => String(row)) : [];
}

function writeBridgeOutput(out) {
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  }
}

function bridgeStatusCode(out) {
  const parsed = Number(out && out.status);
  return Number.isFinite(parsed) ? parsed : 1;
}

function run(args = process.argv.slice(2)) {
  const normalizedArgs = normalizeArgs(args);
  const command = commandNameFromArgs(normalizedArgs, DEFAULT_COMMAND);
  const policy = validateMemoryPolicy(normalizedArgs, { command, lane: SYSTEM_ID });
  let out;
  if (!policy.ok) {
    out = guardFailureResult(policy, { system_id: SYSTEM_ID, command });
  } else {
    const isolation = validateSessionIsolation(normalizedArgs, { command, lane: SYSTEM_ID });
    if (!isolation.ok) {
      out = sessionFailureResult(isolation, { system_id: SYSTEM_ID, command });
    } else {
      out = bridge.run([`--system-id=${SYSTEM_ID}`].concat(normalizedArgs));
    }
  }

  writeBridgeOutput(out);
  return out;
}

if (require.main === module) {
  const out = run(process.argv.slice(2));
  process.exit(bridgeStatusCode(out));
}

module.exports = {
  BRIDGE_PATH,
  DEFAULT_COMMAND,
  LANE_DOMAIN,
  lane: bridge.lane,
  systemId: SYSTEM_ID,
  run,
  normalizeArgs,
  writeBridgeOutput,
  bridgeStatusCode,
};
