#!/usr/bin/env node
'use strict';
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const SYSTEM_ID = 'SYSTEMS-SENSORY-ABSTAIN_UNCERTAINTY_CONTRACT';
const bridge = createOpsLaneBridge(__dirname, 'abstain_uncertainty_contract', 'runtime-systems', {
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
  const out = bridge.run([`--system-id=${SYSTEM_ID}`].concat(Array.isArray(args) ? args : []));
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
