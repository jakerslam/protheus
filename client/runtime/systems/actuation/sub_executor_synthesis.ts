#!/usr/bin/env node
'use strict';
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const SYSTEM_ID = 'SYSTEMS-ACTUATION-SUB_EXECUTOR_SYNTHESIS';
const bridge = createOpsLaneBridge(__dirname, 'sub_executor_synthesis', 'runtime-systems', {
  inheritStdio: true
});

function writeStream(stream, chunk) {
  if (chunk) stream.write(chunk);
}

function statusCode(out) {
  const parsed = Number(out && out.status);
  return Number.isFinite(parsed) ? parsed : 1;
}

function run(args = process.argv.slice(2)) {
  const out = bridge.run([`--system-id=${SYSTEM_ID}`].concat(Array.isArray(args) ? args : []));
  writeStream(process.stdout, out && out.stdout);
  writeStream(process.stderr, out && out.stderr);
  if (out && out.payload && !out.stdout) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  }
  return out;
}

if (require.main === module) {
  const out = run(process.argv.slice(2));
  process.exit(statusCode(out));
}

module.exports = {
  lane: bridge.lane,
  systemId: SYSTEM_ID,
  run
};
