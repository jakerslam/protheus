#!/usr/bin/env node
'use strict';

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const SYSTEM_ID = 'SYSTEMS-ASSIMILATION-SOURCE_ATTESTATION_EXTENSION';
const bridge = createOpsLaneBridge(__dirname, 'source_attestation_extension', 'runtime-systems', {
  inheritStdio: true
});

function run(args = process.argv.slice(2)) {
  const normalizedArgs = Array.isArray(args) ? args.map((row) => String(row)) : [];
  const out = bridge.run([`--system-id=${SYSTEM_ID}`].concat(normalizedArgs));
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  }
  return out;
}

if (require.main === module) {
  const out = run(process.argv.slice(2));
  process.exit(Number.isFinite(Number(out && out.status)) ? Number(out && out.status) : 1);
}

module.exports = {
  lane: bridge.lane,
  systemId: SYSTEM_ID,
  run
};
