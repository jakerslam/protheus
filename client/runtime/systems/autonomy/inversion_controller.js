#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops::inversion-controller (authoritative)
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge');
const tsBootstrap = require('../../lib/ts_bootstrap');
process.env.PROTHEUS_CONDUIT_STARTUP_PROBE = '0';

const bridge = createOpsLaneBridge(__dirname, 'inversion_controller', 'inversion-controller');

function shouldFallback(out) {
  const reason = String((out && out.payload && out.payload.reason) || out && out.stderr || '');
  return /conduit_/i.test(reason) || /timeout/i.test(reason);
}

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
  const status = Number.isFinite(out && out.status) ? Number(out.status) : 1;
  if (status !== 0 && shouldFallback(out)) {
    tsBootstrap.bootstrap(__filename, module);
    return;
  }
  process.exit(status);
}

module.exports = {
  lane: bridge.lane,
  run: bridge.run
};
