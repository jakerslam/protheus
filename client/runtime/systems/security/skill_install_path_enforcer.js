#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/security::skill-install-path-enforcer (authoritative)
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge');

process.env.PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS || '1500';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '2000';

const SECURITY_CMD = 'skill-install-path-enforcer';
const bridge = createOpsLaneBridge(__dirname, 'skill_install_path_enforcer', 'security-plane');

function runCore(args = []) {
  const out = bridge.run([SECURITY_CMD, ...(Array.isArray(args) ? args : [])]);
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
  run: (args = []) => runCore(args)
};
