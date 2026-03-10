#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/spine::rsi_idle_hands_scheduler (authoritative)
// Thin wrapper only; authority logic lives in core/layer2/spine.
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge');

process.env.PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS || '1200';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '1500';

const bridge = createOpsLaneBridge(__dirname, 'rsi_idle_hands_scheduler', 'spine');
const COMMAND = 'rsi-idle-hands-scheduler';

function runCore(args = []) {
  const out = bridge.run([COMMAND, ...(Array.isArray(args) ? args : [])]);
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  return out;
}

if (require.main === module) {
  const out = runCore(process.argv.slice(2));
  process.exit(Number.isFinite(out && out.status) ? Number(out.status) : 1);
}

module.exports = {
  lane: bridge.lane,
  run: (args = []) => bridge.run([COMMAND, ...(Array.isArray(args) ? args : [])])
};
