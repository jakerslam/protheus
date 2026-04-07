#!/usr/bin/env node
'use strict';
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const SYSTEM_ID = 'SYSTEMS-BUDGET-CAPITAL_ALLOCATION_ORGAN';
const bridge = createOpsLaneBridge(__dirname, 'capital_allocation_organ', 'runtime-systems', {
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
