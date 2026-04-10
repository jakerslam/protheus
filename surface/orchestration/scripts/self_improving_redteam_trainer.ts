#!/usr/bin/env node
'use strict';
// Orchestration Surface coordination implementation (non-canonical).
// Layer ownership: surface/orchestration.

const { createOpsLaneBridge } = require('../../../adapters/runtime/ops_lane_bridge.ts');

const SYSTEM_ID = 'SYSTEMS-REDTEAM-SELF_IMPROVING_REDTEAM_TRAINER';
const bridge = createOpsLaneBridge(__dirname, 'self_improving_redteam_trainer', 'runtime-systems', {
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
