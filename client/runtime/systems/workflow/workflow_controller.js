#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/autonomy + core/layer0/ops::workflow-controller (authoritative)
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge');

process.env.PROTHEUS_CONDUIT_STARTUP_PROBE = '0';
process.env.PROTHEUS_CONDUIT_STARTUP_PROBE_TIMEOUT_MS =
  process.env.PROTHEUS_CONDUIT_STARTUP_PROBE_TIMEOUT_MS || '8000';
process.env.PROTHEUS_OPS_LOCAL_FALLBACK = process.env.PROTHEUS_OPS_LOCAL_FALLBACK || '0';
process.env.PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS || '12000';

const bridge = createOpsLaneBridge(__dirname, 'workflow_controller', 'workflow-controller');

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
