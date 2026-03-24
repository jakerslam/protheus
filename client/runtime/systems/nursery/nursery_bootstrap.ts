#!/usr/bin/env node
'use strict';
const fs = require('fs');
const path = require('path');
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const repoRoot = path.resolve(__dirname, '../../../../');
const localOpsBin = path.join(repoRoot, 'target', 'debug', 'protheus-ops');
if (fs.existsSync(localOpsBin)) process.env.PROTHEUS_OPS_BIN = localOpsBin;

const SYSTEM_ID = 'V6-OPENCLAW-DETACH-001.1';
const bridge = createOpsLaneBridge(__dirname, 'nursery_bootstrap', 'runtime-systems', {
  inheritStdio: true,
  preferLocalCore: true
});

function run(args = process.argv.slice(2)) {
  const passthrough = Array.isArray(args) ? args.slice() : [];
  if (!passthrough.some((row) => String(row).startsWith('--strict='))) passthrough.push('--strict=1');
  if (!passthrough.some((row) => String(row).startsWith('--apply='))) passthrough.push('--apply=1');
  if (!passthrough.some((row) => String(row).startsWith('--payload-json='))) {
    const sourceRoot = process.env.INFRING_OPENCLAW_SOURCE_ROOT || '..';
    passthrough.push(`--payload-json=${JSON.stringify({ source_root: sourceRoot })}`);
  }
  const out = bridge.run(['run', `--system-id=${SYSTEM_ID}`].concat(passthrough));
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
  systemId: SYSTEM_ID,
  run
};
