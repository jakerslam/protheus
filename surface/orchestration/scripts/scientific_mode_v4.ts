#!/usr/bin/env node
'use strict';
// Orchestration Surface coordination implementation (non-canonical).
// Layer ownership: surface/orchestration.

const { createOpsLaneBridge } = require('../../../client/runtime/lib/rust_lane_bridge.ts');

const SYSTEM_ID = 'SYSTEMS-SCIENCE-SCIENTIFIC_MODE_V4';
const bridge = createOpsLaneBridge(__dirname, 'scientific_mode_v4', 'runtime-systems', {
  inheritStdio: true
});

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
  run
};
