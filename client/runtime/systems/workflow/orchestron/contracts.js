#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/autonomy + core/layer0/ops::workflow-controller (authoritative)
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge');

const bridge = createOpsLaneBridge(__dirname, 'orchestron_contracts', 'workflow-controller');

function run(args = []) {
  const mapped = String(args[0] || '').trim().toLowerCase() === 'help' ? ['help'] : ['status'];
  return bridge.run(mapped);
}

if (require.main === module) {
  const out = run(process.argv.slice(2));
  if (out && out.stdout) process.stdout.write(out.stdout);
  else if (out && out.payload) process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  if (out && out.stderr) process.stderr.write(out.stderr);
  process.exit(Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1);
}

module.exports = { run };
