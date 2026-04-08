#!/usr/bin/env node
'use strict';

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');
const { commandNameFromArgs, validateMemoryPolicy, guardFailureResult } = require('./policy_validator.ts');
const { validateSessionIsolation, sessionFailureResult } = require('./session_isolation.ts');

const SYSTEM_ID = 'SYSTEMS-MEMORY-RUST_MEMORY_TRANSITION_LANE';
const bridge = createOpsLaneBridge(__dirname, 'rust_memory_transition_lane', 'runtime-systems', {
  inheritStdio: true
});

function run(args = process.argv.slice(2)) {
  const normalizedArgs = Array.isArray(args) ? args.map((row) => String(row)) : [];
  const command = commandNameFromArgs(normalizedArgs, 'status');
  const policy = validateMemoryPolicy(normalizedArgs, {
    command,
    lane: SYSTEM_ID
  });
  let out;
  if (!policy.ok) {
    out = guardFailureResult(policy, {
      command,
      system_id: SYSTEM_ID
    });
  } else {
    const isolation = validateSessionIsolation(normalizedArgs, {
      command,
      lane: SYSTEM_ID
    });
    if (!isolation.ok) {
      out = sessionFailureResult(isolation, {
        command,
        system_id: SYSTEM_ID
      });
    } else {
      out = bridge.run([`--system-id=${SYSTEM_ID}`].concat(normalizedArgs));
    }
  }
  return out;
}

if (require.main === module) {
  const out = run(process.argv.slice(2));
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  }
  process.exit(Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1);
}

module.exports = {
  lane: bridge.lane,
  systemId: SYSTEM_ID,
  run
};
