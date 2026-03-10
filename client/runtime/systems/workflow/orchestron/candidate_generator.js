#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/autonomy + core/layer0/ops::workflow-controller (authoritative)
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge');

const bridge = createOpsLaneBridge(__dirname, 'orchestron_candidate_generator', 'workflow-controller');

function mapArgs(argv = []) {
  const cmd = String(argv[0] || 'list').trim().toLowerCase();
  if (cmd === 'run') return ['list', ...argv.slice(1)];
  if (cmd === 'status') return ['status', ...argv.slice(1)];
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') return ['help'];
  return ['list', ...argv];
}

function run(args = []) {
  return bridge.run(mapArgs(args));
}

if (require.main === module) {
  const out = run(process.argv.slice(2));
  if (out && out.stdout) process.stdout.write(out.stdout);
  else if (out && out.payload) process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  if (out && out.stderr) process.stderr.write(out.stderr);
  process.exit(Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1);
}

module.exports = { run };
