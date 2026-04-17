#!/usr/bin/env node
'use strict';
export {};

const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..');
const TARGET = path.join(ROOT, 'systems', 'ops', 'open_platform_release_pack.js');
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
  const proc = spawnSync(process.execPath, [TARGET].concat(passthrough), {
    cwd: ROOT,
    env: process.env,
    stdio: 'inherit',
  });

  if (proc && proc.error) {
    process.stderr.write(
      `${JSON.stringify({
        ok: false,
        type: 'export_cli',
        error: 'export_cli_spawn_failed',
        detail: String(proc.error && proc.error.message ? proc.error.message : proc.error),
        status: 1
      })}\n`
    );
  }

  const code = Number.isFinite(Number(proc && proc.status)) ? Number(proc.status) : 1;
  process.exit(code);
}

main(process.argv.slice(2));
