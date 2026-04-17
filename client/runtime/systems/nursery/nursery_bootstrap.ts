#!/usr/bin/env node
'use strict';
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const SYSTEM_ID = 'V6-INFRING-DETACH-001.1';
const MAX_ARG_LEN = 512;
const bridge = createOpsLaneBridge(__dirname, 'nursery_bootstrap', 'runtime-systems', {
  inheritStdio: true,
  preferLocalCore: true
});

function sanitizeArg(value) {
  return String(value == null ? '' : value)
    .replace(/[\u200B\u200C\u200D\u2060\uFEFF]/g, '')
    .replace(/[\r\n\t]+/g, ' ')
    .replace(/[^\x20-\x7E]+/g, '')
    .trim()
    .slice(0, MAX_ARG_LEN);
}

function run(args = process.argv.slice(2)) {
  const passthrough = Array.isArray(args)
    ? args.map((arg) => sanitizeArg(arg)).filter(Boolean)
    : [];
  if (!passthrough.some((row) => String(row).startsWith('--strict='))) passthrough.push('--strict=1');
  if (!passthrough.some((row) => String(row).startsWith('--apply='))) passthrough.push('--apply=1');
  if (!passthrough.some((row) => String(row).startsWith('--payload-json='))) {
    const sourceRoot = sanitizeArg(process.env.INFRING_INFRING_SOURCE_ROOT || '..') || '..';
    passthrough.push(`--payload-json=${JSON.stringify({ source_root: sourceRoot })}`);
  }
  const out = bridge.run(['run', `--system-id=${SYSTEM_ID}`].concat(passthrough));
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  } else if (!out || (!out.stdout && !out.stderr)) {
    process.stdout.write(
      `${JSON.stringify({
        ok: false,
        type: 'nursery_bootstrap',
        error: 'bridge_no_output',
        status: Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1
      })}\n`
    );
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
