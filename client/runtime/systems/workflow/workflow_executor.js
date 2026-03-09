#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops::workflow-executor (authoritative)
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge');
process.env.PROTHEUS_CONDUIT_STARTUP_PROBE = '0';

const bridge = createOpsLaneBridge(__dirname, 'workflow_executor', 'workflow-executor');

function runCore(args) {
  const out = bridge.run(args);
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  return out;
}

if (require.main === module) {
  const args = process.argv.slice(2);
  const out = runCore(args);
  process.exit(Number.isFinite(out && out.status) ? Number(out.status) : 1);
}

module.exports = {
  lane: bridge.lane,
  run: bridge.run
};
