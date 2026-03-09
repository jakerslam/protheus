#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/autonomy + core/layer0/ops::autonomy-controller (authoritative)
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge');

const bridge = createOpsLaneBridge(__dirname, 'autonomy_controller', 'autonomy-controller');

function toCoreArgs(argv = []) {
  const args = Array.isArray(argv) ? argv.slice() : [];
  const cmd = String(args[0] || 'status').trim().toLowerCase();
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') return ['help'];
  const allowed = new Set(['run', 'status']);
  const action = allowed.has(cmd) ? cmd : 'status';
  const tail = args.slice(args.length > 0 ? 1 : 0);
  return ['self-documentation-closeout', `--action=${action}`, ...tail];
}

function run(args = []) {
  return bridge.run(toCoreArgs(args));
}

if (require.main === module) {
  const out = run(process.argv.slice(2));
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  process.exit(Number.isFinite(out && out.status) ? Number(out.status) : 1);
}

module.exports = {
  lane: bridge.lane,
  run
};
