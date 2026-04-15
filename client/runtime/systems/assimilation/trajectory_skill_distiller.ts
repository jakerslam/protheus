#!/usr/bin/env node
'use strict';

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const SYSTEM_ID = 'SYSTEMS-ASSIMILATION-TRAJECTORY_SKILL_DISTILLER';
const bridge = createOpsLaneBridge(__dirname, 'trajectory_skill_distiller', 'runtime-systems', {
  inheritStdio: true
});

function normalizeArgs(args) {
  return Array.isArray(args) ? args.map((row) => String(row == null ? '' : row)) : [];
}

function emitBridgeOutput(out) {
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  }
}

function run(args = process.argv.slice(2)) {
  const normalizedArgs = normalizeArgs(args);
  const out = bridge.run([`--system-id=${SYSTEM_ID}`].concat(normalizedArgs));
  emitBridgeOutput(out);
  return out;
}

function runCli(args = process.argv.slice(2)) {
  const out = run(args);
  return Number.isFinite(Number(out && out.status)) ? Number(out && out.status) : 1;
}

if (require.main === module) {
  process.exit(runCli(process.argv.slice(2)));
}

module.exports = {
  lane: bridge.lane,
  systemId: SYSTEM_ID,
  run,
  runCli
};
