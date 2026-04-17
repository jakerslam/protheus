#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-022 open-platform compatibility surface.
 * Delegates to canonical systems/economy/public_donation_api lane.
 */

const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const API_SCRIPT = path.join(ROOT, 'systems', 'economy', 'public_donation_api.ts');
const MAX_ARG_LEN = 512;

function sanitizeArg(value) {
  return String(value == null ? '' : value)
    .replace(/[\u200B\u200C\u200D\u2060\uFEFF]/g, '')
    .replace(/[\r\n\t]+/g, ' ')
    .replace(/[^\x20-\x7E]+/g, '')
    .trim()
    .slice(0, MAX_ARG_LEN);
}

function main(args = process.argv.slice(2)) {
  const passthrough = Array.isArray(args) ? args.map((arg) => sanitizeArg(arg)).filter(Boolean) : [];
  const proc = spawnSync(process.execPath, [API_SCRIPT, ...passthrough], {
    cwd: ROOT,
    encoding: 'utf8',
    env: process.env,
  });
  if (proc && proc.stdout) process.stdout.write(String(proc.stdout));
  if (proc && proc.stderr) process.stderr.write(String(proc.stderr));
  if ((!proc || (!proc.stdout && !proc.stderr)) && !(proc && proc.error)) {
    process.stdout.write(
      `${JSON.stringify({
        ok: false,
        type: 'donate_gpu',
        error: 'bridge_no_output',
        status: Number.isFinite(Number(proc && proc.status)) ? Number(proc.status) : 1
      })}\n`
    );
  }
  if (proc && proc.error) {
    process.stderr.write(
      `${JSON.stringify({
        ok: false,
        type: 'donate_gpu',
        error: 'donate_gpu_spawn_failed',
        detail: String(proc.error && proc.error.message ? proc.error.message : proc.error),
        status: 1
      })}\n`
    );
  }
  process.exit(Number.isFinite(Number(proc && proc.status)) ? Number(proc.status) : 1);
}

main(process.argv.slice(2));
