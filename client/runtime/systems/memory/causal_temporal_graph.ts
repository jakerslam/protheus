#!/usr/bin/env node
'use strict';
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const bridge = createOpsLaneBridge(__dirname, 'causal_temporal_graph', 'memory-plane', {
  inheritStdio: true
});

function mapArgs(args = []) {
  const rows = Array.isArray(args) ? args.map((v) => String(v)) : [];
  const cmd = String(rows[0] || '').trim().toLowerCase();
  if (!cmd || cmd === 'status' || cmd === 'verify') {
    return ['status'].concat(rows.slice(cmd ? 1 : 0));
  }
  if (cmd === 'build') {
    return [
      'record',
      '--event-id=build-latest',
      '--summary=legacy_build_alias',
      '--actor=system',
      '--apply=0'
    ].concat(rows.slice(1));
  }
  if (cmd === 'query') {
    return ['blame', '--event-id=build-latest'].concat(rows.slice(1));
  }
  return rows;
}

function run(args = process.argv.slice(2)) {
  const out = bridge.run(['causal-temporal-graph'].concat(mapArgs(args)));
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
  run
};
