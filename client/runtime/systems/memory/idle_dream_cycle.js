#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::memory-ambient (authoritative)
const { runMemoryAmbientCommand } = require('../../lib/spine_conduit_bridge');
const path = require('path');
const { spawnSync } = require('child_process');

const TS_ENTRYPOINT = path.resolve(__dirname, '..', '..', '..', 'lib', 'ts_entrypoint.js');
const LEGACY_TS = path.resolve(__dirname, 'legacy', 'idle_dream_cycle_legacy.ts');

function usage() {
  console.log('Usage:');
  console.log('  node systems/memory/idle_dream_cycle.js run [YYYY-MM-DD] [--force=1] [--rem-only=1]');
  console.log('  node systems/memory/idle_dream_cycle.js status');
}

function toAmbientArgs(argv) {
  const cmd = String((argv && argv[0]) || '').trim().toLowerCase();
  if (!cmd || cmd === 'run') {
    const tail = (argv || []).slice(cmd ? 1 : 0);
    return ['run', 'idle-dream-cycle', ...tail];
  }
  if (cmd === 'status') return ['status'];
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') return ['run', 'help'];
  return ['run', ...argv];
}

async function run(args = [], opts = {}) {
  const mapped = toAmbientArgs(args);
  const ambientEnabled = String(process.env.PROTHEUS_IDLE_DREAM_AMBIENT || '0').trim() === '1';
  if (ambientEnabled) {
    try {
      const out = await runMemoryAmbientCommand(mapped, {
        runContext: 'idle_dream_cycle_wrapper',
        skipRuntimeGate: true,
        stdioTimeoutMs: Number(process.env.PROTHEUS_MEMORY_STDIO_TIMEOUT_MS || 25000),
        ...opts
      });
      if (out && out.ok === true && out.payload && out.payload.ok !== false) {
        return out;
      }
    } catch {
      // compatibility fallback below
    }
  }
  const proc = spawnSync(process.execPath, [TS_ENTRYPOINT, LEGACY_TS, ...args], {
    cwd: process.cwd(),
    encoding: 'utf8',
    env: process.env
  });
  const payloadText = String(proc.stdout || '').trim();
  let payload = null;
  if (payloadText) {
    try { payload = JSON.parse(payloadText); } catch {}
  }
  return {
    ok: Number(proc.status || 0) === 0 && payload && payload.ok !== false,
    status: Number.isFinite(Number(proc.status)) ? Number(proc.status) : 1,
    payload,
    stdout: String(proc.stdout || ''),
    stderr: String(proc.stderr || '')
  };
}

if (require.main === module) {
  const raw = process.argv.slice(2);
  const token = String(raw[0] || '').trim().toLowerCase();
  if (token === 'help' || token === '--help' || token === '-h') {
    usage();
    process.exit(0);
  }
  process.env.PROTHEUS_CONDUIT_STARTUP_PROBE = '0';
  process.env.PROTHEUS_CONDUIT_COMPAT_FALLBACK = '0';
  process.env.PROTHEUS_CONDUIT_STARTUP_PROBE_TIMEOUT_MS =
    process.env.PROTHEUS_CONDUIT_STARTUP_PROBE_TIMEOUT_MS || '8000';
  run(raw)
    .then((out) => {
      if (out && out.payload) process.stdout.write(`${JSON.stringify(out.payload)}\n`);
      if (out && out.stderr) process.stderr.write(out.stderr.endsWith('\n') ? out.stderr : `${out.stderr}\n`);
      process.exit(Number.isFinite(out && out.status) ? Number(out.status) : 1);
    })
    .catch((error) => {
      process.stdout.write(
        `${JSON.stringify({
          ok: false,
          type: 'idle_dream_cycle_wrapper_error',
          error: String(error && error.message ? error.message : error)
        })}\n`
      );
      process.exit(1);
    });
}

module.exports = {
  run
};
