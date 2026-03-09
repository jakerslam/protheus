#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::memory-ambient (authoritative)
const { runMemoryAmbientCommand } = require('../../lib/spine_conduit_bridge');

function toAmbientArgs(argv = []) {
  const args = Array.isArray(argv) ? argv.slice() : [];
  const cmd = String(args[0] || 'status').trim().toLowerCase();
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') return ['run', 'help'];
  if (cmd === 'status') return ['status'];
  const allowed = new Set(['run', 'verify', 'restore']);
  const action = allowed.has(cmd) ? cmd : 'run';
  const tail = args.slice(args.length > 0 ? 1 : 0);
  return ['run', 'cryonics-tier', `--action=${action}`, ...tail];
}

async function run(args = [], opts = {}) {
  return runMemoryAmbientCommand(toAmbientArgs(args), {
    runContext: 'cryonics_tier_wrapper',
    stdioTimeoutMs: Number(process.env.PROTHEUS_MEMORY_STDIO_TIMEOUT_MS || 25000),
    ...opts
  });
}

if (require.main === module) {
  process.env.PROTHEUS_CONDUIT_STARTUP_PROBE = '0';
  process.env.PROTHEUS_CONDUIT_COMPAT_FALLBACK = '0';
  process.env.PROTHEUS_CONDUIT_STARTUP_PROBE_TIMEOUT_MS =
    process.env.PROTHEUS_CONDUIT_STARTUP_PROBE_TIMEOUT_MS || '8000';
  run(process.argv.slice(2))
    .then((out) => {
      if (out && out.payload) process.stdout.write(`${JSON.stringify(out.payload)}\n`);
      if (out && out.stderr) process.stderr.write(out.stderr.endsWith('\n') ? out.stderr : `${out.stderr}\n`);
      process.exit(Number.isFinite(out && out.status) ? Number(out.status) : 1);
    })
    .catch((error) => {
      process.stdout.write(
        `${JSON.stringify({
          ok: false,
          type: 'cryonics_tier_wrapper_error',
          error: String(error && error.message ? error.message : error)
        })}\n`
      );
      process.exit(1);
    });
}

module.exports = {
  run
};
