#!/usr/bin/env node
'use strict';

const { createManifestLaneBridge } = require('../../lib/rust_lane_bridge.ts');
const { commandNameFromArgs, validateMemoryPolicy, guardFailureResult } = require('./policy_validator.ts');
const { validateSessionIsolation, sessionFailureResult } = require('./session_isolation.ts');

const bridge = createManifestLaneBridge(__dirname, 'rust_memory_transition_lane', {
  manifestPath: 'client/runtime/systems/memory/rust/Cargo.toml',
  binaryName: 'rust_memory_transition_lane',
  binaryEnvVar: 'PROTHEUS_MEMORY_TRANSITION_RUST_BIN',
  inheritStdio: true
});

function run(args = process.argv.slice(2)) {
  const normalizedArgs = Array.isArray(args) ? args.map((row) => String(row)) : [];
  const command = commandNameFromArgs(normalizedArgs, 'status');
  const policy = validateMemoryPolicy(normalizedArgs, {
    command,
    lane: 'SYSTEMS-MEMORY-RUST_MEMORY_TRANSITION_LANE'
  });
  let out;
  if (!policy.ok) {
    out = guardFailureResult(policy, {
      command,
      system_id: 'SYSTEMS-MEMORY-RUST_MEMORY_TRANSITION_LANE'
    });
  } else {
    const isolation = validateSessionIsolation(normalizedArgs, {
      command,
      lane: 'SYSTEMS-MEMORY-RUST_MEMORY_TRANSITION_LANE'
    });
    if (!isolation.ok) {
      out = sessionFailureResult(isolation, {
        command,
        system_id: 'SYSTEMS-MEMORY-RUST_MEMORY_TRANSITION_LANE'
      });
    } else {
      out = bridge.run(normalizedArgs);
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
  run
};
