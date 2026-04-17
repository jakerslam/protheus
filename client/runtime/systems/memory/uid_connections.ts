#!/usr/bin/env node
'use strict';
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');
const { commandNameFromArgs, validateMemoryPolicy, guardFailureResult } = require('./policy_validator.ts');
const { validateSessionIsolation, sessionFailureResult } = require('./session_isolation.ts');

const SYSTEM_ID = 'SYSTEMS-MEMORY-UID_CONNECTIONS';
const bridge = createOpsLaneBridge(__dirname, 'uid_connections', 'runtime-systems', {
  inheritStdio: true
});
const FORBIDDEN_RUNTIME_CONTEXT_MARKERS = [
  'You are an expert Python programmer.',
  '[PATCH v2',
  'List Leaves (25',
  'BEGIN_OPENCLAW_INTERNAL_CONTEXT',
  'END_OPENCLAW_INTERNAL_CONTEXT',
  'UNTRUSTED_CHILD_RESULT_DELIMITER'
];

function containsForbiddenRuntimeContextMarker(raw = '') {
  const text = String(raw);
  return FORBIDDEN_RUNTIME_CONTEXT_MARKERS.some((marker) => text.includes(marker));
}

function run(args = process.argv.slice(2)) {
  const normalizedArgs = Array.isArray(args) ? args.map((row) => String(row)) : [];
  const command = commandNameFromArgs(normalizedArgs, 'status');
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
  run,
  forbiddenRuntimeContextMarkers: FORBIDDEN_RUNTIME_CONTEXT_MARKERS,
  containsForbiddenRuntimeContextMarker
};
